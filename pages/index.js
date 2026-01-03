import { useState, useRef, useEffect, useCallback } from 'react';
import Head from 'next/head';

export default function Home() {
    const [scanner, setScanner] = useState(null);
    const [wasmReady, setWasmReady] = useState(false);
    const [results, setResults] = useState([]);
    const [status, setStatus] = useState('Loading WASM...');
    const [mode, setMode] = useState('upload'); // 'camera' or 'upload'
    const [scanning, setScanning] = useState(false);

    const videoRef = useRef(null);
    const canvasRef = useRef(null);
    const streamRef = useRef(null);
    const intervalRef = useRef(null);

    // Load WASM on mount
    useEffect(() => {
        async function loadWasm() {
            try {
                const wasm = await import('/pkg/qr_wasm.js');
                await wasm.default();
                const scannerInstance = new wasm.WasmQRScanner();
                setScanner(scannerInstance);
                setWasmReady(true);
                setStatus('Ready');
            } catch (error) {
                console.error('WASM load error:', error);
                setStatus('WASM load failed: ' + error.message);
            }
        }
        loadWasm();

        return () => {
            stopCamera();
        };
    }, []);

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

        setStatus('Processing...');

        try {
            const arrayBuffer = await file.arrayBuffer();
            const uint8Array = new Uint8Array(arrayBuffer);
            const result = scanner.scanImage(uint8Array);

            if (result && result.qr_codes) {
                setResults(result.qr_codes);
                setStatus(`Found ${result.qr_codes.length} QR code(s) in ${result.processing_time_ms}ms`);
            } else {
                setResults([]);
                setStatus('No QR codes found');
            }
        } catch (error) {
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
