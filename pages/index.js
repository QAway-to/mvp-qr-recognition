import { useState, useRef, useEffect, useCallback } from 'react';
import Head from 'next/head';

// Global debug setup - hook console to capture Rust/WASM logs
if (typeof window !== 'undefined') {
    window.__QR_DEBUG_LOGS = window.__QR_DEBUG_LOGS || [];

    if (!window.__CONSOLE_HOOKED) {
        window.__CONSOLE_HOOKED = true;
        const methods = ['log', 'info', 'warn', 'error', 'debug'];
        methods.forEach(method => {
            const original = console[method];
            console[method] = function (...args) {
                if (window.__QR_DEBUG_LOGS) {
                    window.__QR_DEBUG_LOGS.push({
                        t: new Date().toISOString(),
                        cat: 'CONSOLE_' + method.toUpperCase(),
                        msg: args.map(a => String(a)).join(' '),
                        data: null
                    });
                }
                original.apply(console, args);
            };
        });
    }
}

function log(cat, msg, data = null) {
    if (typeof window !== 'undefined') {
        window.__QR_DEBUG_LOGS.push({ t: new Date().toISOString(), cat, msg, data });
    }
    console.info(`[${cat}] ${msg}`, data || '');
}

export default function Home() {
    const [scanner, setScanner] = useState(null);
    const [wasmReady, setWasmReady] = useState(false);
    const [results, setResults] = useState([]);
    const [status, setStatus] = useState('Loading WASM...');
    const [mode, setMode] = useState('upload');
    const [scanning, setScanning] = useState(false);
    const [batchResults, setBatchResults] = useState([]);
    const [mlLoaded, setMlLoaded] = useState(false);

    const videoRef = useRef(null);
    const canvasRef = useRef(null);
    const streamRef = useRef(null);
    const intervalRef = useRef(null);
    const wasmModuleRef = useRef(null);

    const loadWasm = async () => {
        try {
            log('WASM', 'Fetching JS module');

            // Fetch the WASM binary first
            const wasmResponse = await fetch('/pkg/qr_wasm_bg.wasm');
            log('WASM', 'WASM fetch status', wasmResponse.status);

            if (!wasmResponse.ok) {
                throw new Error(`Failed to fetch WASM: ${wasmResponse.status}`);
            }

            // Import the JS glue as ES module using dynamic import with data URL
            log('WASM', 'Fetching JS glue');
            const jsResponse = await fetch('/pkg/qr_wasm.js');
            const jsText = await jsResponse.text();
            log('WASM', 'JS length', jsText.length);

            // Convert to base64 data URL for import
            const base64 = btoa(unescape(encodeURIComponent(jsText)));
            const dataUrl = `data:application/javascript;base64,${base64}`;

            log('WASM', 'Importing module');
            const wasmModule = await import(/* webpackIgnore: true */ dataUrl);
            log('WASM', 'Module keys', Object.keys(wasmModule));

            wasmModuleRef.current = wasmModule;

            // Initialize with the fetch response
            log('WASM', 'Calling init with fetch response');
            await wasmModule.default(wasmResponse);
            log('WASM', 'Init complete');

            // Create scanner
            if (wasmModule.WasmQRScanner) {
                log('WASM', 'Creating scanner');
                const scannerInstance = new wasmModule.WasmQRScanner();
                setScanner(scannerInstance);
                setWasmReady(true);
                setStatus('Ready');
                log('WASM', 'Ready!');

                // Try to load ML model
                try {
                    log('ML', 'Fetching model.onnx...');
                    const modelResp = await fetch('/model.onnx');
                    if (modelResp.ok) {
                        const modelBuf = await modelResp.arrayBuffer();
                        const modelBytes = new Uint8Array(modelBuf);
                        scannerInstance.loadModel(modelBytes);
                        setMlLoaded(true);
                        log('ML', 'Model loaded successfully (YOLOv8)');
                    } else {
                        log('ML', 'Model not found (using Fallback)');
                    }
                } catch (e) {
                    log('ML', 'Failed to load model', e);
                }
            } else {
                log('WASM', 'ERROR: No WasmQRScanner in exports');
                setStatus('Error: WasmQRScanner not found');
            }
        } catch (error) {
            log('WASM', 'ERROR', { name: error.name, message: error.message, stack: error.stack });
            setStatus('Error: ' + error.message);
        }
    };

    const scanFrame = useCallback(() => {
        if (!scanner || !videoRef.current || !canvasRef.current) return;
        const video = videoRef.current;
        const canvas = canvasRef.current;
        const ctx = canvas.getContext('2d');
        canvas.width = video.videoWidth;
        canvas.height = video.videoHeight;
        ctx.drawImage(video, 0, 0);
        const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
        try {
            const result = scanner.scanImageData(imageData.data, canvas.width, canvas.height);
            if (result?.qr_codes?.length > 0) {
                setResults(result.qr_codes);
                setStatus(`Found ${result.qr_codes.length} QR code(s)`);
            }
        } catch (e) {
            log('SCAN', 'Frame error', e.message);
        }
    }, [scanner]);

    const stopCamera = useCallback(() => {
        if (intervalRef.current) {
            clearInterval(intervalRef.current);
            intervalRef.current = null;
        }
        if (streamRef.current) {
            streamRef.current.getTracks().forEach(track => track.stop());
            streamRef.current = null;
        }
        setScanning(false);
    }, []);

    const startCamera = async () => {
        try {
            setStatus('Starting camera...');
            const stream = await navigator.mediaDevices.getUserMedia({
                video: { facingMode: 'environment', width: { ideal: 1280 }, height: { ideal: 720 } }
            });
            streamRef.current = stream;
            if (videoRef.current) {
                videoRef.current.srcObject = stream;
                videoRef.current.onloadedmetadata = () => {
                    setScanning(true);
                    setStatus('Scanning...');
                    intervalRef.current = setInterval(scanFrame, 300);
                };
            }
        } catch (error) {
            log('CAMERA', 'ERROR', error.message);
            setStatus('Camera error: ' + error.message);
        }
    };

    const handleFileUpload = async (event) => {
        const file = event.target.files[0];
        if (!file || !scanner) return;
        log('UPLOAD', 'File', { name: file.name, size: file.size, type: file.type });
        setStatus('Processing...');
        try {
            log('UPLOAD', 'Reading file as ArrayBuffer');
            const arrayBuffer = await file.arrayBuffer();
            log('UPLOAD', 'ArrayBuffer size', arrayBuffer.byteLength);

            const uint8Array = new Uint8Array(arrayBuffer);
            log('UPLOAD', 'Uint8Array created', { length: uint8Array.length, first4bytes: Array.from(uint8Array.slice(0, 4)) });

            log('UPLOAD', 'Scanner state', { hasScanner: !!scanner, scannerType: typeof scanner });
            log('UPLOAD', 'Calling scanner.scanImage...');

            const result = scanner.scanImage(uint8Array);

            log('UPLOAD', 'scanImage returned', { hasResult: !!result, resultType: typeof result });
            log('UPLOAD', 'Result details', result);

            if (result?.qr_codes) {
                setResults(result.qr_codes);
                setStatus(`Found ${result.qr_codes.length} QR code(s)`);
            } else {
                setResults([]);
                setStatus('No QR codes found');
            }
        } catch (error) {
            log('UPLOAD', 'ERROR caught', {
                name: error.name,
                message: error.message,
                stack: error.stack?.substring(0, 500)
            });
            setStatus('Error: ' + error.message);
        }
    };

    const handleBatchUpload = async (event) => {
        const files = Array.from(event.target.files);
        if (!files.length || !scanner) return;

        setBatchResults([]);
        setStatus(`Processing ${files.length} files...`);

        const results = [];
        let success = 0;

        for (const file of files) {
            try {
                const arrayBuffer = await file.arrayBuffer();
                const uint8Array = new Uint8Array(arrayBuffer);
                const start = performance.now();
                const result = scanner.scanImage(uint8Array);
                const time = performance.now() - start;

                const hasQr = result?.qr_codes?.length > 0;
                if (hasQr) success++;

                results.push({
                    name: file.name,
                    status: hasQr ? '✅ Found' : '❌ No QR',
                    content: hasQr ? result.qr_codes[0].content : '-',
                    time: Math.round(time)
                });

                // Update periodically
                setBatchResults([...results]);
            } catch (e) {
                results.push({ name: file.name, status: '⚠️ Error', content: e.message, time: 0 });
                setBatchResults([...results]);
            }
        }

        setStatus(`Done. Success: ${success}/${files.length} (${Math.round(success / files.length * 100)}%)`);
    };

    const downloadLogs = () => {
        const text = JSON.stringify(window.__QR_DEBUG_LOGS || [], null, 2);
        const blob = new Blob([text], { type: 'application/json' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = 'qr-scanner-logs.json';
        a.click();
        URL.revokeObjectURL(url);
    };

    useEffect(() => {
        log('INIT', 'Starting WASM load');
        loadWasm();
        return () => stopCamera();
    }, []);

    return (
        <>
            <Head>
                <title>QR Scanner</title>
                <meta name="viewport" content="width=device-width, initial-scale=1" />
            </Head>

            <main className="container">
                <h1>📱 QR Scanner</h1>
                <p className="subtitle">WASM-powered QR code recognition</p>

                <div className="mode-toggle">
                    <button className={mode === 'camera' ? 'active' : ''} onClick={() => { setMode('camera'); stopCamera(); }}>
                        📷 Camera
                    </button>
                    <button className={mode === 'upload' ? 'active' : ''} onClick={() => { setMode('upload'); stopCamera(); }}>
                        📁 Upload
                    </button>
                    <button className={mode === 'batch' ? 'active' : ''} onClick={() => { setMode('batch'); stopCamera(); }}>
                        📚 Batch Test
                    </button>
                </div>

                {mode === 'camera' && (
                    <div className="camera-section">
                        <video ref={videoRef} autoPlay playsInline muted />
                        <canvas ref={canvasRef} style={{ display: 'none' }} />
                        <div className="controls">
                            {!scanning ? (
                                <button onClick={startCamera} disabled={!wasmReady} className="btn-primary">Start Camera</button>
                            ) : (
                                <button onClick={stopCamera} className="btn-secondary">Stop Camera</button>
                            )}
                        </div>
                    </div>
                )}

                {mode === 'upload' && (
                    <div className="upload-section">
                        <label className="dropzone">
                            <input type="file" accept="image/*" onChange={handleFileUpload} disabled={!wasmReady} />
                            <span>📷 Click or drop SINGLE image here</span>
                        </label>
                    </div>
                )}

                {mode === 'batch' && (
                    <div className="batch-section">
                        <label className="dropzone">
                            <input type="file" accept="image/*" multiple onChange={handleBatchUpload} disabled={!wasmReady} />
                            <span>📚 Select MULTIPLE files for Mass Test</span>
                        </label>

                        {batchResults.length > 0 && (
                            <div className="batch-results">
                                <table style={{ width: '100%', marginTop: '20px', borderCollapse: 'collapse' }}>
                                    <thead>
                                        <tr style={{ textAlign: 'left', borderBottom: '1px solid #333' }}>
                                            <th>File</th>
                                            <th>Status</th>
                                            <th>Time (ms)</th>
                                            <th>Content</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {batchResults.map((res, i) => (
                                            <tr key={i} style={{ borderBottom: '1px solid #222' }}>
                                                <td>{res.name}</td>
                                                <td>{res.status}</td>
                                                <td>{res.time}</td>
                                                <td style={{ maxWidth: '200px', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{res.content}</td>
                                            </tr>
                                        ))}
                                    </tbody>
                                </table>
                            </div>
                        )}
                    </div>
                )}

                <div className="status">
                    Status: <strong>{status}</strong>
                    <button onClick={downloadLogs} style={{ marginLeft: '10px', fontSize: '0.8rem', padding: '4px 8px' }}>
                        📥 Logs
                    </button>
                </div>

                {results.length > 0 && (
                    <div className="results">
                        <h3>Results</h3>
                        {results.map((qr, idx) => (
                            <div key={idx} className={`result-card ${qr.content_type === 'Payment' ? 'payment' : ''}`}>
                                <div className="result-type">{qr.content_type}</div>
                                <div className="result-content">{qr.content}</div>
                                {qr.payment && (
                                    <div className="payment-info">
                                        {qr.payment.payee_name && <div>Recipient: {qr.payment.payee_name}</div>}
                                        {qr.payment.amount && <div>Amount: {qr.payment.amount} {qr.payment.currency}</div>}
                                    </div>
                                )}
                            </div>
                        ))}
                    </div>
                )}
            </main>
        </>
    );
}
