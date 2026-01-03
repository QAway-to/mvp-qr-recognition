// QR Scanner Demo - Main JavaScript

import init, { WasmQRScanner, quickScan, version } from '../pkg/qr_wasm.js';

let scanner = null;
let videoStream = null;
let scanInterval = null;
let currentCamera = 'environment';

// DOM Elements
const elements = {
    video: document.getElementById('video'),
    overlay: document.getElementById('overlay'),
    cameraView: document.getElementById('cameraView'),
    uploadView: document.getElementById('uploadView'),
    startBtn: document.getElementById('startBtn'),
    switchBtn: document.getElementById('switchBtn'),
    dropzone: document.getElementById('dropzone'),
    fileInput: document.getElementById('fileInput'),
    previewCanvas: document.getElementById('previewCanvas'),
    resultsContent: document.getElementById('resultsContent'),
    processingTime: document.getElementById('processingTime'),
    qrCount: document.getElementById('qrCount'),
    status: document.getElementById('status'),
    versionEl: document.getElementById('version'),
    // Settings
    adaptiveThreshold: document.getElementById('adaptiveThreshold'),
    denoise: document.getElementById('denoise'),
    enhanceContrast: document.getElementById('enhanceContrast'),
    blockSize: document.getElementById('blockSize'),
    blockSizeValue: document.getElementById('blockSizeValue'),
};

// Initialize WASM module
async function initWasm() {
    try {
        await init();
        scanner = new WasmQRScanner();
        elements.versionEl.textContent = version();
        updateStatus('Ready');
        console.log('WASM Scanner initialized');
    } catch (error) {
        console.error('Failed to initialize WASM:', error);
        updateStatus('Init Error', true);
    }
}

// Update status display
function updateStatus(text, isError = false) {
    elements.status.textContent = text;
    elements.status.style.color = isError ? 'var(--error)' : 'var(--accent-secondary)';
}

// Mode switching
document.querySelectorAll('.mode-btn').forEach(btn => {
    btn.addEventListener('click', () => {
        document.querySelectorAll('.mode-btn').forEach(b => b.classList.remove('active'));
        btn.classList.add('active');

        const mode = btn.dataset.mode;
        if (mode === 'camera') {
            elements.cameraView.classList.remove('hidden');
            elements.uploadView.classList.add('hidden');
        } else {
            elements.cameraView.classList.add('hidden');
            elements.uploadView.classList.remove('hidden');
            stopCamera();
        }
    });
});

// Camera controls
elements.startBtn.addEventListener('click', toggleCamera);
elements.switchBtn.addEventListener('click', switchCamera);

async function toggleCamera() {
    if (videoStream) {
        stopCamera();
    } else {
        await startCamera();
    }
}

async function startCamera() {
    try {
        updateStatus('Starting camera...');

        const constraints = {
            video: {
                facingMode: currentCamera,
                width: { ideal: 1280 },
                height: { ideal: 720 }
            }
        };

        videoStream = await navigator.mediaDevices.getUserMedia(constraints);
        elements.video.srcObject = videoStream;

        elements.startBtn.textContent = 'Stop Camera';
        elements.switchBtn.style.display = 'block';

        // Start scanning loop
        elements.video.onloadedmetadata = () => {
            scanInterval = setInterval(scanFrame, 200);
            updateStatus('Scanning...');
        };

    } catch (error) {
        console.error('Camera error:', error);
        updateStatus('Camera Error', true);
    }
}

function stopCamera() {
    if (videoStream) {
        videoStream.getTracks().forEach(track => track.stop());
        videoStream = null;
    }

    if (scanInterval) {
        clearInterval(scanInterval);
        scanInterval = null;
    }

    elements.video.srcObject = null;
    elements.startBtn.textContent = 'Start Camera';
    elements.switchBtn.style.display = 'none';
    updateStatus('Ready');
}

async function switchCamera() {
    currentCamera = currentCamera === 'environment' ? 'user' : 'environment';
    stopCamera();
    await startCamera();
}

// Scan current video frame
async function scanFrame() {
    if (!scanner || !elements.video.videoWidth) return;

    const canvas = document.createElement('canvas');
    const ctx = canvas.getContext('2d');

    canvas.width = elements.video.videoWidth;
    canvas.height = elements.video.videoHeight;
    ctx.drawImage(elements.video, 0, 0);

    const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);

    try {
        const result = scanner.scanImageData(
            imageData.data,
            canvas.width,
            canvas.height
        );

        if (result && result.qr_codes && result.qr_codes.length > 0) {
            displayResults(result);
            drawOverlay(result, canvas.width, canvas.height);
        }
    } catch (error) {
        console.error('Scan error:', error);
    }
}

// File upload handling
elements.dropzone.addEventListener('click', () => elements.fileInput.click());
elements.fileInput.addEventListener('change', handleFileSelect);

elements.dropzone.addEventListener('dragover', (e) => {
    e.preventDefault();
    elements.dropzone.classList.add('dragover');
});

elements.dropzone.addEventListener('dragleave', () => {
    elements.dropzone.classList.remove('dragover');
});

elements.dropzone.addEventListener('drop', (e) => {
    e.preventDefault();
    elements.dropzone.classList.remove('dragover');

    const files = e.dataTransfer.files;
    if (files.length > 0) {
        processFile(files[0]);
    }
});

function handleFileSelect(e) {
    const file = e.target.files[0];
    if (file) {
        processFile(file);
    }
}

async function processFile(file) {
    if (!scanner) {
        updateStatus('Scanner not ready', true);
        return;
    }

    updateStatus('Processing...');

    try {
        const arrayBuffer = await file.arrayBuffer();
        const uint8Array = new Uint8Array(arrayBuffer);

        const result = scanner.scanImage(uint8Array);

        // Show preview
        const img = new Image();
        img.onload = () => {
            elements.previewCanvas.classList.remove('hidden');
            elements.previewCanvas.width = img.width;
            elements.previewCanvas.height = img.height;

            const ctx = elements.previewCanvas.getContext('2d');
            ctx.drawImage(img, 0, 0);

            // Draw QR boxes
            if (result && result.qr_codes) {
                ctx.strokeStyle = '#6c5ce7';
                ctx.lineWidth = 3;

                result.qr_codes.forEach(qr => {
                    const [x, y, w, h] = qr.bbox;
                    ctx.strokeRect(x, y, w, h);
                });
            }
        };
        img.src = URL.createObjectURL(file);

        displayResults(result);
        updateStatus('Done');

    } catch (error) {
        console.error('Process error:', error);
        updateStatus('Error: ' + error.message, true);
    }
}

// Display scan results
function displayResults(result) {
    if (!result || !result.qr_codes || result.qr_codes.length === 0) {
        elements.resultsContent.innerHTML = '<p class="placeholder">No QR codes found</p>';
        elements.qrCount.textContent = '0';
        return;
    }

    elements.processingTime.textContent = result.processing_time_ms + 'ms';
    elements.qrCount.textContent = result.qr_codes.length;

    let html = '';

    result.qr_codes.forEach((qr, idx) => {
        const isPayment = qr.content_type === 'Payment';
        const isBest = result.best_payment === idx;

        html += `
            <div class="qr-result ${isPayment ? 'payment' : ''}">
                <div class="qr-result-header">
                    <span class="qr-result-type ${isPayment ? 'payment' : ''}">${qr.content_type}</span>
                    ${isBest ? '<span class="qr-result-type payment">💳 Best Payment</span>' : ''}
                </div>
                <div class="qr-result-content">${escapeHtml(qr.content)}</div>
                ${qr.payment ? renderPaymentDetails(qr.payment) : ''}
            </div>
        `;
    });

    elements.resultsContent.innerHTML = html;
}

function renderPaymentDetails(payment) {
    let html = '<div class="payment-details">';

    if (payment.payee_name) {
        html += `<div class="payment-detail">
            <span class="payment-detail-label">Recipient</span>
            <span class="payment-detail-value">${escapeHtml(payment.payee_name)}</span>
        </div>`;
    }

    if (payment.amount) {
        const currency = payment.currency || 'RUB';
        html += `<div class="payment-detail">
            <span class="payment-detail-label">Amount</span>
            <span class="payment-detail-value">${payment.amount} ${currency}</span>
        </div>`;
    }

    if (payment.bank) {
        html += `<div class="payment-detail">
            <span class="payment-detail-label">Bank</span>
            <span class="payment-detail-value">${escapeHtml(payment.bank)}</span>
        </div>`;
    }

    if (payment.purpose) {
        html += `<div class="payment-detail">
            <span class="payment-detail-label">Purpose</span>
            <span class="payment-detail-value">${escapeHtml(payment.purpose)}</span>
        </div>`;
    }

    html += '</div>';
    return html;
}

// Draw overlay on video
function drawOverlay(result, width, height) {
    const canvas = elements.overlay;
    const ctx = canvas.getContext('2d');

    canvas.width = width;
    canvas.height = height;

    ctx.clearRect(0, 0, width, height);

    if (!result || !result.qr_codes) return;

    const scaleX = canvas.clientWidth / width;
    const scaleY = canvas.clientHeight / height;

    result.qr_codes.forEach((qr, idx) => {
        const [x, y, w, h] = qr.bbox;
        const isPayment = qr.content_type === 'Payment';
        const isBest = result.best_payment === idx;

        ctx.strokeStyle = isBest ? '#00d9a5' : (isPayment ? '#00d9a5' : '#6c5ce7');
        ctx.lineWidth = 3;
        ctx.strokeRect(x * scaleX, y * scaleY, w * scaleX, h * scaleY);

        // Label
        ctx.fillStyle = ctx.strokeStyle;
        ctx.font = 'bold 14px Inter, sans-serif';
        ctx.fillText(qr.content_type, x * scaleX, (y - 5) * scaleY);
    });
}

// Settings handlers
elements.blockSize.addEventListener('input', (e) => {
    elements.blockSizeValue.textContent = e.target.value;
    recreateScanner();
});

[elements.adaptiveThreshold, elements.denoise, elements.enhanceContrast].forEach(el => {
    el.addEventListener('change', recreateScanner);
});

function recreateScanner() {
    if (!scanner) return;

    scanner = WasmQRScanner.withConfig(
        elements.adaptiveThreshold.checked,
        parseInt(elements.blockSize.value),
        elements.denoise.checked,
        1.0,
        elements.enhanceContrast.checked
    );
}

// Utility
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

// Initialize
initWasm();
