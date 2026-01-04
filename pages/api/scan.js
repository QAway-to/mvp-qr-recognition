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

        // Use the synchronous or async scan method
        // scan_image returns a JS object with result
        // We create a new scanner instance for each request to ensure isolation
        const scanner = new wasm.WasmQRScanner();

        // Intercept console logs to capture WASM output
        const logs = [];
        const originalInfo = console.info;
        const originalLog = console.log;
        const originalError = console.error;
        const originalDebug = console.debug;

        const captureLog = (type, args) => {
            const msg = args.map(a => (typeof a === 'object' ? JSON.stringify(a) : String(a))).join(' ');
            logs.push(`[${type.toUpperCase()}] ${msg}`);
            // Also print to real stdout for Vercel logs
            if (type === 'error') originalError.apply(console, args);
            else originalInfo.apply(console, args);
        };

        console.info = (...args) => captureLog('info', args);
        console.log = (...args) => captureLog('info', args);
        console.error = (...args) => captureLog('error', args);
        console.debug = (...args) => captureLog('debug', args);

        let result;
        try {
            // Note: scan_image expects Uint8Array
            result = scanner.scan_image(bytes);
        } catch (e) {
            console.error('WASM Scan Error:', e);
            // Restore console before returning error
            console.info = originalInfo;
            console.log = originalLog;
            console.error = originalError;
            console.debug = originalDebug;
            return res.status(500).json({ error: 'Scanning failed', details: e.toString(), logs });
        }

        // Restore console
        console.info = originalInfo;
        console.log = originalLog;
        console.error = originalError;
        console.debug = originalDebug;

        return res.status(200).json({ ...result, logs });

    } catch (error) {
        console.error('API Error:', error);
        return res.status(500).json({ error: error.message });
    }
}
