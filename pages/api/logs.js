// API endpoint to retrieve debug logs
// Access: GET /api/logs

export default function handler(req, res) {
    // This is a server-side endpoint, but logs are client-side
    // Return instructions for getting logs
    res.status(200).json({
        message: "Logs are stored client-side. Open browser console and run: JSON.stringify(window.__QR_DEBUG_LOGS, null, 2)",
        alternativeEndpoint: "/api/logs-fetch (requires client to POST logs)"
    });
}
