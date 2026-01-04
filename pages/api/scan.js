// Simple GET-only endpoint for version check and log download

export default async function handler(req, res) {
    // Only GET allowed
    if (req.method !== 'GET') {
        return res.status(405).json({ error: 'Method not allowed. Use GET.' });
    }

    // Return version and status as downloadable JSON
    return res.status(200).json({
        status: 'ready',
        version: 'V16',
        timestamp: new Date().toISOString(),
        logs: ['QR Scanner API is operational.']
    });
}
