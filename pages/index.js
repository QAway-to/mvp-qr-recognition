import { useState, useRef, useEffect, useCallback } from 'react';
import Head from 'next/head';

// Debug logs
const logs = [];
function log(cat, msg, data = null) {
    const entry = { t: new Date().toISOString(), cat, msg, data };
    logs.push(entry);
    console.log(`[${cat}] ${msg}`, data || '');
}

export default function Home() {
    const [scanner, setScanner] = useState(null);
    const [wasmReady, setWasmReady] = useState(false);
    const [results, setResults] = useState([]);
    const [status, setStatus] = useState('Loading WASM...');
    const [mode, setMode] = useState('upload');
    const [scanning, setScanning] = useState(false);

    const videoRef = useRef(null);
    const canvasRef = useRef(null);
    const streamRef = useRef(null);
    const intervalRef = useRef(null);
    const wasmModuleRef = useRef(null);

    useEffect(() => {
        log('INIT', 'Starting WASM load');
        loadWasm();
        return () => stopCamera();
    }, []);

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

            // Initialize with the fetch response (this is supported per line 458-459)
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
            } else {
                log('WASM', 'ERROR: No WasmQRScanner in exports');
                setStatus('Error: WasmQRScanner not found');
            }
        } catch (error) {
            log('WASM', 'ERROR', { name: error.name, message: error.message, stack: error.stack });
            setStatus('Error: ' + error.message);
        }
    };

    const downloadLogs = () => {
        const text = JSON.stringify(logs, null, 2);
        const blob = new Blob([text], { type: 'application/json' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = 'qr-scanner-logs.json';
        a.click();
        URL.revokeObjectURL(url);
    };

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
        } catch (e) { /* silent */ }
    }, [scanner]);

    const handleFileUpload = async (event) => {
        const file = event.target.files[0];
        if (!file || !scanner) return;
        log('UPLOAD', 'File', { name: file.name, size: file.size });
        setStatus('Processing...');
        try {
            const arrayBuffer = await file.arrayBuffer();
            const uint8Array = new Uint8Array(arrayBuffer);
            const result = scanner.scanImage(uint8Array);
            log('UPLOAD', 'Result', result);
            if (result?.qr_codes) {
                setResults(result.qr_codes);
                setStatus(`Found ${result.qr_codes.length} QR code(s)`);
            } else {
                setResults([]);
                setStatus('No QR codes found');
            }
        } catch (error) {
            log('UPLOAD', 'ERROR', error.message);
            setStatus('Error: ' + error.message);
        }
    };

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
                            <span>📷 Click or drop image here</span>
                        </label>
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
