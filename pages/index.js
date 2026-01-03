import { useState, useRef, useEffect, useCallback } from 'react';
import Head from 'next/head';
import Script from 'next/script';

// Global logs array accessible from /api/logs
if (typeof window !== 'undefined') {
    window.__QR_DEBUG_LOGS = window.__QR_DEBUG_LOGS || [];
}

function debugLog(category, message, data = null) {
    const timestamp = new Date().toISOString();
    const logEntry = { timestamp, category, message, data: data ? JSON.stringify(data) : null };

    console.log(`[${timestamp}] [${category}] ${message}`, data || '');

    if (typeof window !== 'undefined') {
        window.__QR_DEBUG_LOGS.push(logEntry);
        // Keep only last 100 logs
        if (window.__QR_DEBUG_LOGS.length > 100) {
            window.__QR_DEBUG_LOGS.shift();
        }
    }
}

export default function Home() {
    const [scanner, setScanner] = useState(null);
    const [wasmReady, setWasmReady] = useState(false);
    const [results, setResults] = useState([]);
    const [status, setStatus] = useState('Loading WASM...');
    const [mode, setMode] = useState('upload');
    const [scanning, setScanning] = useState(false);
    const [scriptLoaded, setScriptLoaded] = useState(false);

    const videoRef = useRef(null);
    const canvasRef = useRef(null);
    const streamRef = useRef(null);
    const intervalRef = useRef(null);

    useEffect(() => {
        debugLog('INIT', 'Component mounted');
        debugLog('INIT', 'Window object', {
            hasWindow: typeof window !== 'undefined',
            hasWasmBindgen: typeof window !== 'undefined' && !!window.wasm_bindgen
        });

        return () => {
            stopCamera();
        };
    }, []);

    // Initialize WASM after script loads
    const initWasm = async () => {
        debugLog('WASM', 'Script onLoad triggered');
        debugLog('WASM', 'Checking wasm_bindgen availability', {
            hasWasmBindgen: typeof window !== 'undefined' && !!window.wasm_bindgen,
            wasmBindgenType: typeof window !== 'undefined' ? typeof window.wasm_bindgen : 'N/A'
        });

        try {
            if (typeof window !== 'undefined' && window.wasm_bindgen) {
                debugLog('WASM', 'wasm_bindgen found, initializing...');

                const wasmUrl = '/pkg/qr_wasm_bg.wasm';
                debugLog('WASM', 'Fetching WASM from URL', { url: wasmUrl });

                await window.wasm_bindgen(wasmUrl);
                debugLog('WASM', 'wasm_bindgen() completed successfully');

                debugLog('WASM', 'Available exports', {
                    keys: Object.keys(window.wasm_bindgen)
                });

                if (window.wasm_bindgen.WasmQRScanner) {
                    debugLog('WASM', 'Creating WasmQRScanner instance');
                    const scannerInstance = new window.wasm_bindgen.WasmQRScanner();
                    debugLog('WASM', 'Scanner instance created', { scanner: !!scannerInstance });

                    setScanner(scannerInstance);
                    setWasmReady(true);
                    setStatus('Ready');
                    debugLog('WASM', 'Initialization complete - Ready');
                } else {
                    debugLog('WASM', 'ERROR: WasmQRScanner not found in exports');
                    setStatus('Error: WasmQRScanner not found');
                }
            } else {
                debugLog('WASM', 'ERROR: wasm_bindgen not available on window');
                setStatus('Error: wasm_bindgen not loaded');
            }
        } catch (error) {
            debugLog('WASM', 'ERROR during initialization', {
                name: error.name,
                message: error.message,
                stack: error.stack
            });
            setStatus('Error: ' + error.message);
        }
    };

    const onScriptLoad = () => {
        debugLog('SCRIPT', 'Script element loaded');
        setScriptLoaded(true);

        // Small delay to ensure script is fully executed
        setTimeout(() => {
            debugLog('SCRIPT', 'Calling initWasm after delay');
            initWasm();
        }, 100);
    };

    const onScriptError = (e) => {
        debugLog('SCRIPT', 'Script loading ERROR', { error: e?.toString() });
        setStatus('Failed to load WASM script');
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
            debugLog('CAMERA', 'Starting camera...');
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
            debugLog('CAMERA', 'ERROR', { message: error.message });
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
            if (result && result.qr_codes && result.qr_codes.length > 0) {
                setResults(result.qr_codes);
                setStatus(`Found ${result.qr_codes.length} QR code(s) in ${result.processing_time_ms}ms`);
            }
        } catch (error) {
            console.error('Scan error:', error);
        }
    }, [scanner]);

    const handleFileUpload = async (event) => {
        const file = event.target.files[0];
        if (!file || !scanner) return;

        debugLog('UPLOAD', 'File selected', { name: file.name, size: file.size, type: file.type });
        setStatus('Processing...');

        try {
            const arrayBuffer = await file.arrayBuffer();
            const uint8Array = new Uint8Array(arrayBuffer);
            debugLog('UPLOAD', 'Calling scanner.scanImage', { byteLength: uint8Array.length });

            const result = scanner.scanImage(uint8Array);
            debugLog('UPLOAD', 'Scan result', result);

            if (result && result.qr_codes) {
                setResults(result.qr_codes);
                setStatus(`Found ${result.qr_codes.length} QR code(s) in ${result.processing_time_ms}ms`);
            } else {
                setResults([]);
                setStatus('No QR codes found');
            }
        } catch (error) {
            debugLog('UPLOAD', 'ERROR', { message: error.message, stack: error.stack });
            setStatus('Error: ' + error.message);
        }
    };

    return (
        <>
            <Head>
                <title>QR Scanner</title>
                <meta name="description" content="QR Code Scanner with WASM" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
            </Head>

            {/* Load WASM JS file */}
            <Script
                src="/pkg/qr_wasm.js"
                strategy="afterInteractive"
                onLoad={onScriptLoad}
                onError={onScriptError}
            />

            <main className="container">
                <h1>📱 QR Scanner</h1>
                <p className="subtitle">WASM-powered QR code recognition</p>

                {/* Mode Toggle */}
                <div className="mode-toggle">
                    <button
                        className={mode === 'camera' ? 'active' : ''}
                        onClick={() => { setMode('camera'); stopCamera(); }}
                    >
                        📷 Camera
                    </button>
                    <button
                        className={mode === 'upload' ? 'active' : ''}
                        onClick={() => { setMode('upload'); stopCamera(); }}
                    >
                        📁 Upload
                    </button>
                </div>

                {/* Camera Mode */}
                {mode === 'camera' && (
                    <div className="camera-section">
                        <video ref={videoRef} autoPlay playsInline muted />
                        <canvas ref={canvasRef} style={{ display: 'none' }} />
                        <div className="controls">
                            {!scanning ? (
                                <button onClick={startCamera} disabled={!wasmReady} className="btn-primary">
                                    Start Camera
                                </button>
                            ) : (
                                <button onClick={stopCamera} className="btn-secondary">
                                    Stop Camera
                                </button>
                            )}
                        </div>
                    </div>
                )}

                {/* Upload Mode */}
                {mode === 'upload' && (
                    <div className="upload-section">
                        <label className="dropzone">
                            <input type="file" accept="image/*" onChange={handleFileUpload} disabled={!wasmReady} />
                            <span>📷 Click or drop image here</span>
                        </label>
                    </div>
                )}

                {/* Status */}
                <div className="status">
                    Status: <strong>{status}</strong>
                </div>

                {/* Results */}
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
