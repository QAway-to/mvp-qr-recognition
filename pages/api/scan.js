import { IncomingForm } from 'formidable';
import fs from 'fs';
import path from 'path';

// Disable standard body parser to handle file uploads
export const config = {
    api: {
        bodyParser: false,
    },
};

// Lazy load WASM to prevent build issues
let wasmModule = null;

async function loadWasm() {
    if (!wasmModule) {
        // Import from the local library path we copied pkg-node to
        // Note: Dynamic import path must be relative or alias
        const wasm = await import('../../lib/pkg-node/qr_wasm.js');
        wasmModule = wasm;
    }
    return wasmModule;
}

export default async function handler(req, res) {
    // Handle GET for health check or server-side file scan
    if (req.method === 'GET') {
        const { file, list } = req.query;

        // List available files
        if (list) {
            const fs = require('fs');
            const path = require('path');
            const dir = path.join(process.cwd(), 'generated_dataset');
            try {
                const files = fs.readdirSync(dir);
                return res.status(200).json({ files });
            } catch (e) {
                return res.status(500).json({ error: 'Failed to list files', details: e.message });
            }
        }

        // Scan specific file
        if (file) {
            const wasm = await loadWasm();
            const logs = [];
            const originalLog = console.log;
            const originalInfo = console.info;
            const originalWarn = console.warn;
            const originalError = console.error;

            const capture = (...args) => {
                logs.push({
                    t: new Date().toISOString(),
                    cat: 'SERVER_LOG',
                    msg: args.map(a => (typeof a === 'object' ? JSON.stringify(a) : String(a))).join(' ')
                });
                // Optional: still log to real console
                // originalLog.apply(console, args); 
            };

            console.log = capture;
            console.info = capture;
            console.warn = capture;
            console.error = capture;

            try {
                const fs = require('fs');
                const path = require('path');
                const filePath = path.join(process.cwd(), 'generated_dataset', file);

                if (!fs.existsSync(filePath)) {
                    return res.status(404).json({ error: 'File not found', filePath });
                }

                const buffer = fs.readFileSync(filePath);
                const bytes = new Uint8Array(buffer);

                const scanner = new wasm.WasmQRScanner();
                const result = scanner.scan_image(bytes);

                return res.status(200).json({
                    file,
                    result,
                    logs
                });

            } catch (e) {
                return res.status(500).json({ error: 'Scan failed', logs, details: e.toString() });
            } finally {
                console.log = originalLog;
                console.info = originalInfo;
                console.warn = originalWarn;
                console.error = originalError;
            }
        }

        return res.status(200).json({
            status: 'ready',
            version: 'V15',
            logs: ['Server is ready. Use ?file=filename.png to scan server-side files.']
        });
    }

    if (req.method !== 'POST') {
        return res.status(405).json({ error: 'Method not allowed' });
    }

    try {
        const form = new IncomingForm();

        const [fields, files] = await new Promise((resolve, reject) => {
            form.parse(req, (err, fields, files) => {
                if (err) reject(err);
                resolve([fields, files]);
            });
        });

        const file = files.file?.[0] || files.image?.[0];
        if (!file) {
            return res.status(400).json({ error: 'No file uploaded' });
        }

        const buffer = fs.readFileSync(file.filepath);
        const bytes = new Uint8Array(buffer);

        console.log(`Processing image: ${file.originalFilename}, size: ${bytes.length}`);

        const wasm = await loadWasm();

        // Intercept console logs to return them in the response
        const logs = [];
        const originalLog = console.log;
        const originalInfo = console.info;
        const originalWarn = console.warn;
        const originalError = console.error;

        function intercept(args) {
            logs.push(args.map(a => String(a)).join(' '));
        }

        console.log = (...args) => { intercept(args); originalLog.apply(console, args); };
        console.info = (...args) => { intercept(args); originalInfo.apply(console, args); };
        console.warn = (...args) => { intercept(args); originalWarn.apply(console, args); };
        // We probably don't want to swallow errors, but we can capture them
        console.error = (...args) => { intercept(args); originalError.apply(console, args); };

        // Use the synchronous or async scan method
        // scan_image returns a JS object with result
        // We create a new scanner instance for each request to ensure isolation
        const scanner = new wasm.WasmQRScanner();

        let result;
        try {
            // Note: scan_image expects Uint8Array
            result = scanner.scan_image(bytes);
        } catch (e) {
            console.error('WASM Scan Error:', e);
            result = { error: e.toString() };
        } finally {
            // Restore console
            console.log = originalLog;
            console.info = originalInfo;
            console.warn = originalWarn;
            console.error = originalError;
        }

        return res.status(200).json({ result, logs });

    } catch (error) {
        console.error('API Error:', error); // This might use the intercepted console if error happens during interception block, which is fine
        return res.status(500).json({ error: error.message });
    }
}
