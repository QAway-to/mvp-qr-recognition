import { useState, useRef, useEffect, useCallback } from 'react';
import Head from 'next/head';
import dynamic from 'next/dynamic';
import jsQR from 'jsqr';

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

async function flushLogs() {
    if (typeof window === 'undefined') return;
    try {
        await fetch('/api/logs', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                logs: window.__QR_DEBUG_LOGS,
                clientInfo: {
                    ua: navigator.userAgent,
                    ts: new Date().toISOString(),
                }
            })
        });
    } catch (_) {}
}

export default function Home() {
    // ===== ML DETECTION TOGGLE =====
    // Set to true to enable ML-based QR detection (JS onnxruntime-web, ~300ms)
    // Set to false to use only algorithmic detection (fast, stable)
    const ENABLE_ML_DETECTION = true;

    const [scanner, setScanner] = useState(null);
    const [wasmReady, setWasmReady] = useState(false);
    const [results, setResults] = useState([]);
    const [status, setStatus] = useState('Loading WASM...');
    const [mode, setMode] = useState('upload');
    const [scanning, setScanning] = useState(false);
    const [batchResults, setBatchResults] = useState([]);
    const [mlLoaded, setMlLoaded] = useState(false);
    // ML init state machine: 'idle' -> 'loading' -> 'ready' | 'failed'
    const [mlStatus, setMlStatus] = useState(ENABLE_ML_DETECTION ? 'idle' : 'disabled');

    const videoRef = useRef(null);
    const canvasRef = useRef(null);
    const streamRef = useRef(null);
    const intervalRef = useRef(null);
    const wasmModuleRef = useRef(null);
    const mlDetectorRef = useRef(null);  // JS-based ML detector
    const mlStatusRef = useRef(mlStatus);
    const mlInitPromiseRef = useRef(null);

    useEffect(() => {
        mlStatusRef.current = mlStatus;
    }, [mlStatus]);

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
                flushLogs();

                // Try to load ML model using JS onnxruntime-web (only if enabled)
                if (ENABLE_ML_DETECTION) {
                    setMlStatus('loading');
                    mlStatusRef.current = 'loading';

                    // Start ML init once. Scans can await a short race against this promise if needed.
                    mlInitPromiseRef.current = (async () => {
                        try {
                            log('ML', 'Loading onnxruntime-web MLDetector...');
                            const { MLDetector } = await import('../lib/mlDetector.js');
                            const detector = new MLDetector();
                            const loaded = await detector.loadModel('/model.onnx');

                            if (loaded) {
                                mlDetectorRef.current = detector;
                                setMlLoaded(true);
                                setMlStatus('ready');
                                mlStatusRef.current = 'ready';
                                log('ML', 'JS MLDetector ready (onnxruntime-web)');
                                return true;
                            }

                            setMlLoaded(false);
                            setMlStatus('failed');
                            mlStatusRef.current = 'failed';
                            log('ML', 'Failed to load model (loaded=false)');
                            return false;
                        } catch (e) {
                            setMlLoaded(false);
                            setMlStatus('failed');
                            mlStatusRef.current = 'failed';
                            log('ML', 'Failed to init MLDetector', e?.message || String(e));
                            return false;
                        }
                    })();
                } else {
                    log('ML', 'ML detection disabled (ENABLE_ML_DETECTION = false)');
                }
            } else {
                log('WASM', 'ERROR: No WasmQRScanner in exports');
                setStatus('Error: WasmQRScanner not found');
            }
        } catch (error) {
            log('WASM', 'ERROR', { name: error.name, message: error.message, stack: error.stack });
            setStatus('Error: ' + error.message);
            flushLogs();
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

    // Unified processing logic for both single and batch uploads
    const processImageWithFallback = async (file) => {
        if (!scanner) throw new Error('Scanner not initialized');

        const arrayBuffer = await file.arrayBuffer();
        const uint8Array = new Uint8Array(arrayBuffer);
        const startTime = performance.now();

        // 1. First pass: Standard algorithmic scan
        let result = scanner.scanImage(uint8Array);

        // 2. Fallback: If no QR found, try ML detection + Cropping
        if ((!result?.qr_codes || result.qr_codes.length === 0)) {
            // Check if we should use ML
            if (ENABLE_ML_DETECTION) {
                // If ML is still initializing, wait a tiny bit (never 10s per image)
                if (mlStatusRef.current === 'loading' && mlInitPromiseRef.current) {
                    await Promise.race([
                        mlInitPromiseRef.current,
                        new Promise((resolve) => setTimeout(resolve, 250))
                    ]);
                }

                // If ML is failed/disabled/not ready: skip ML path immediately
                if (mlStatusRef.current === 'ready' && mlDetectorRef.current) {
                    log('SCAN', 'ML Fallback: Running detection...');

                    try {
                        const blob = new Blob([uint8Array], { type: file.type });
                        const bitmap = await createImageBitmap(blob);

                        // Prepare canvas for ML input
                        const canvas = document.createElement('canvas');
                        canvas.width = bitmap.width;
                        canvas.height = bitmap.height;
                        const ctx = canvas.getContext('2d');
                        ctx.drawImage(bitmap, 0, 0);
                        const imageData = ctx.getImageData(0, 0, bitmap.width, bitmap.height);

                        // Run detection
                        const detections = await mlDetectorRef.current.detect(imageData, 0.65);

                        if (detections.length > 0) {
                            log('SCAN', `ML found ${detections.length} candidate regions`);

                            // For each detection, crop and re-scan
                            for (const det of detections) {
                                // Add padding (20% to ensure quiet zone)
                                const padding = Math.max(det.width, det.height) * 0.2;
                                const x = Math.max(0, det.x - padding);
                                const y = Math.max(0, det.y - padding);
                                const w = Math.min(bitmap.width - x, det.width + padding * 2);
                                const h = Math.min(bitmap.height - y, det.height + padding * 2);

                                log('SCAN', `Re-scanning crop: ${x | 0},${y | 0} ${w | 0}x${h | 0}`);

                                const cropCanvas = document.createElement('canvas');
                                cropCanvas.width = w;
                                cropCanvas.height = h;
                                const cropCtx = cropCanvas.getContext('2d');
                                cropCtx.drawImage(canvas, x, y, w, h, 0, 0, w, h);
                                const cropData = cropCtx.getImageData(0, 0, w, h);

                                // Attempt 0: Try jsQR first on the crop (very robust)
                                const jsQrCode = jsQR(cropData.data, w, h);
                                if (jsQrCode) {
                                    log('SCAN', `✅ jsQR found code: ${jsQrCode.data}`);
                                    result = { qr_codes: [{ content: jsQrCode.data, content_type: "Text" }] };
                                    break;
                                }

                                // Scan the cropped region with WASM
                                let cropResult = scanner.scanImageData(cropData.data, w, h);

                                // ROTATION RETRY: If ML found it but scanner failed, try rotating
                                if (!cropResult?.qr_codes?.length && w > 0 && h > 0) {
                                    log('SCAN', 'Standard scan failed on crop, trying rotation...');

                                    // Try different angles to catch 30, 45, 60 degree rotations (both directions)
                                    const angles = [30, -30, 45, -45, 60, -60];

                                    for (const angle of angles) {
                                        log('SCAN', `Attempting rotation: ${angle}°`);
                                        const rad = (angle * Math.PI) / 180;
                                        const cos = Math.abs(Math.cos(rad));
                                        const sin = Math.abs(Math.sin(rad));

                                        const nw = Math.floor(w * cos + h * sin);
                                        const nh = Math.floor(w * sin + h * cos);

                                        const rotCanvas = document.createElement('canvas');
                                        rotCanvas.width = nw;
                                        rotCanvas.height = nh;
                                        const rotCtx = rotCanvas.getContext('2d');

                                        // CRITICAL FIX: Fill background with WHITE.
                                        // Transparent pixels (0,0,0,0) become black in grayscale conversion,
                                        // destroying the QR quiet zone. We need white padding.
                                        rotCtx.fillStyle = '#FFFFFF';
                                        rotCtx.fillRect(0, 0, nw, nh);

                                        rotCtx.translate(nw / 2, nh / 2);
                                        rotCtx.rotate(rad);
                                        rotCtx.drawImage(cropCanvas, -w / 2, -h / 2);

                                        const rotData = rotCtx.getImageData(0, 0, nw, nh);

                                        // Try jsQR on rotated image
                                        const jsQrRot = jsQR(rotData.data, nw, nh);
                                        if (jsQrRot) {
                                            log('SCAN', `✅ jsQR found code in rotation (${angle}°): ${jsQrRot.data}`);
                                            cropResult = { qr_codes: [{ content: jsQrRot.data, content_type: "Text" }] };
                                            break;
                                        }

                                        // DEBUG: Check center pixel
                                        const cx = Math.floor(nw / 2);
                                        const cy = Math.floor(nh / 2);
                                        const pIdx = (cy * nw + cx) * 4;
                                        log('SCAN', `Rotated center pixel: rgba(${rotData.data[pIdx]},${rotData.data[pIdx + 1]},${rotData.data[pIdx + 2]},${rotData.data[pIdx + 3]})`);

                                        // Attempt 1: Scan normal rotated
                                        let rotResult = scanner.scanImageData(rotData.data, nw, nh);
                                        if (rotResult?.qr_codes?.length > 0) {
                                            log('SCAN', `✅ Success with ${angle}° rotation (normal)!`);
                                            cropResult = rotResult;
                                            break;
                                        }

                                        // Attempt 2: Multi-threshold Binarization
                                        // Try different thresholds to handle lighting variations
                                        const thresholds = [80, 128, 160];

                                        for (const threshold of thresholds) {
                                            // Create a copy of the pixel data for this threshold pass
                                            // We must copy because we are modifying the buffer in place
                                            const pixels = new Uint8ClampedArray(rotData.data);

                                            for (let i = 0; i < pixels.length; i += 4) {
                                                // Simple grayscale
                                                const gray = (pixels[i] + pixels[i + 1] + pixels[i + 2]) / 3;
                                                const val = gray < threshold ? 0 : 255;
                                                pixels[i] = pixels[i + 1] = pixels[i + 2] = val;
                                                // Keep alpha
                                            }

                                            rotResult = scanner.scanImageData(pixels, nw, nh);
                                            if (rotResult?.qr_codes?.length > 0) {
                                                log('SCAN', `✅ Success with ${angle}° rotation (binarized @ ${threshold})!`);
                                                cropResult = rotResult;
                                                break;
                                            }
                                        }

                                        if (cropResult?.qr_codes?.length > 0) break;

                                        // Continue loop if failed...
                                        if (!cropResult?.qr_codes?.length && !window.debugDataUrlLogged) {
                                            log('SCAN', `DEBUG IMAGE (Rotated ${angle}°): ${rotCanvas.toDataURL()}`);
                                            window.debugDataUrlLogged = true; // Log only once to avoid spam
                                        } else if (!cropResult?.qr_codes?.length) {
                                            log('SCAN', `❌ Failed with ${angle}° rotation`);
                                        }
                                    }
                                }

                                if (cropResult?.qr_codes?.length > 0) {
                                    log('SCAN', '✅ Success in cropped region!');
                                    result = cropResult;
                                    break; // Found one, good enough for now
                                }
                            }
                        }
                    } catch (e) {
                        log('SCAN', 'ML processing error', e.message);
                    }
                }
            }
        }

        return {
            result,
            time: performance.now() - startTime
        };
    };

    const handleFileUpload = async (event) => {
        const file = event.target.files[0];
        if (!file || !scanner) return;

        log('UPLOAD', 'File', { name: file.name, size: file.size, type: file.type });
        setStatus('Processing...');
        setResults([]);

        try {
            const { result, time } = await processImageWithFallback(file);

            log('SCAN', `Total scan time: ${time.toFixed(0)}ms`);

            if (result?.qr_codes && result.qr_codes.length > 0) {
                setResults(result.qr_codes);
                setStatus(`Found ${result.qr_codes.length} QR code(s) in ${time.toFixed(0)}ms`);
            } else {
                setResults([]);
                setStatus(`No QR codes found (${time.toFixed(0)}ms)`);
            }
        } catch (error) {
            log('UPLOAD', 'ERROR caught', {
                name: error.name,
                message: error.message
            });
            setStatus('Error: ' + error.message);
        }
    };

    const handleBatchUpload = async (event) => {
        const files = Array.from(event.target.files);
        if (!files.length || !scanner) return;

        setBatchResults([]);
        setStatus(`Processing ${files.length} files...`);

        // If ML is enabled and still loading, wait ONCE (up to 10s) before batch.
        if (ENABLE_ML_DETECTION && mlStatusRef.current === 'loading' && mlInitPromiseRef.current) {
            setStatus('Waiting for ML model to load before batch (up to 10s)...');
            await Promise.race([
                mlInitPromiseRef.current,
                new Promise((resolve) => setTimeout(resolve, 10000))
            ]);
        }

        const results = [];
        let success = 0;

        for (const file of files) {
            try {
                // Use the same robust logic for batch
                const { result, time } = await processImageWithFallback(file);

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

    const mlBadgeClass = mlStatus === 'ready' ? 'ready' : mlStatus === 'loading' ? 'loading' : mlStatus === 'failed' ? 'failed' : '';
    const statusPillClass = !wasmReady ? 'loading' : status.startsWith('Error') ? 'error' : 'ready';

    return (
        <>
            <Head>
                <title>QR Scanner</title>
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <link rel="preconnect" href="https://fonts.googleapis.com" />
                <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
                <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&display=swap" rel="stylesheet" />
            </Head>

            <div className="layout">
                <header className="topbar">
                    <div className="topbar-brand">
                        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                            <rect x="3" y="3" width="7" height="7"/><rect x="14" y="3" width="7" height="7"/>
                            <rect x="3" y="14" width="7" height="7"/><path d="M14 14h3v3h-3zM17 17h3v3h-3zM14 20h3"/>
                        </svg>
                        QR Scanner
                    </div>
                    <div className="topbar-meta">
                        {mlStatus !== 'disabled' && (
                            <span className={`ml-badge ${mlBadgeClass}`}>
                                ML {mlStatus === 'ready' ? 'ready' : mlStatus === 'loading' ? 'loading…' : 'off'}
                            </span>
                        )}
                        <span className={`status-pill ${statusPillClass}`}>
                            <span className={`status-dot ${!wasmReady ? 'pulse' : ''}`} />
                            {status}
                        </span>
                    </div>
                </header>

                <main className="main">
                    <div className="tabs">
                        <button className={`tab ${mode === 'camera' ? 'active' : ''}`} onClick={() => { setMode('camera'); stopCamera(); }}>
                            Camera
                        </button>
                        <button className={`tab ${mode === 'upload' ? 'active' : ''}`} onClick={() => { setMode('upload'); stopCamera(); }}>
                            Upload
                        </button>
                        <button className={`tab ${mode === 'batch' ? 'active' : ''}`} onClick={() => { setMode('batch'); stopCamera(); }}>
                            Batch Test
                        </button>
                    </div>

                    <canvas ref={canvasRef} style={{ display: 'none' }} />

                    {mode === 'camera' && (
                        <div className="card camera-section">
                            <video ref={videoRef} autoPlay playsInline muted />
                            <div className="controls">
                                {!scanning ? (
                                    <button onClick={startCamera} disabled={!wasmReady} className="btn btn-primary">
                                        Start Camera
                                    </button>
                                ) : (
                                    <button onClick={stopCamera} className="btn">
                                        Stop Camera
                                    </button>
                                )}
                            </div>
                        </div>
                    )}

                    {mode === 'upload' && (
                        <label className="dropzone">
                            <input type="file" accept="image/*" onChange={handleFileUpload} disabled={!wasmReady} />
                            <span className="dropzone-icon">
                                <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                                    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" y1="3" x2="12" y2="15"/>
                                </svg>
                            </span>
                            <span className="dropzone-label">Click or drop image here</span>
                            <span className="dropzone-hint">PNG, JPG, WEBP — single file</span>
                        </label>
                    )}

                    {mode === 'batch' && (
                        <div>
                            <label className="dropzone">
                                <input type="file" accept="image/*" multiple onChange={handleBatchUpload} disabled={!wasmReady} />
                                <span className="dropzone-icon">
                                    <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                                        <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" y1="3" x2="12" y2="15"/>
                                    </svg>
                                </span>
                                <span className="dropzone-label">Select multiple files</span>
                                <span className="dropzone-hint">Batch QR recognition test</span>
                            </label>

                            {batchResults.length > 0 && (
                                <div className="batch-table-wrap">
                                    <table className="batch-table">
                                        <thead>
                                            <tr>
                                                <th>File</th>
                                                <th>Status</th>
                                                <th>ms</th>
                                                <th>Content</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {batchResults.map((res, i) => {
                                                const ok = res.status.includes('Found');
                                                const err = res.status.includes('Error');
                                                return (
                                                    <tr key={i}>
                                                        <td>{res.name}</td>
                                                        <td className={ok ? 'badge-ok' : err ? 'badge-err' : 'badge-fail'}>
                                                            {ok ? 'Found' : err ? 'Error' : 'No QR'}
                                                        </td>
                                                        <td>{res.time}</td>
                                                        <td className="truncate">{res.content}</td>
                                                    </tr>
                                                );
                                            })}
                                        </tbody>
                                    </table>
                                </div>
                            )}
                        </div>
                    )}

                    {results.length > 0 && (
                        <div style={{ marginTop: '24px' }}>
                            <div className="results-header">Results — {results.length} code{results.length > 1 ? 's' : ''} found</div>
                            {results.map((qr, idx) => (
                                <div key={idx} className={`result-card ${qr.content_type === 'Payment' ? 'payment' : ''}`}>
                                    <div className="result-type">{qr.content_type || 'Text'}</div>
                                    <div className="result-content">{qr.content}</div>
                                    {qr.payment && (
                                        <div className="payment-info">
                                            {qr.payment.payee_name && (
                                                <div className="payment-row">Recipient <span>{qr.payment.payee_name}</span></div>
                                            )}
                                            {qr.payment.amount && (
                                                <div className="payment-row">Amount <span>{qr.payment.amount} {qr.payment.currency}</span></div>
                                            )}
                                        </div>
                                    )}
                                </div>
                            ))}
                        </div>
                    )}

                    <div style={{ marginTop: '32px', textAlign: 'right' }}>
                        <button onClick={downloadLogs} className="btn" style={{ fontSize: '0.78rem' }}>
                            Download logs
                        </button>
                    </div>
                </main>
            </div>
        </>
    );
}
