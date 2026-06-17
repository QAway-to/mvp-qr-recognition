// In-memory log store (survives within warm Vercel function instance)
const store = { entries: [], updatedAt: null, clientInfo: null };

export default function handler(req, res) {
    if (req.method === 'POST') {
        const { logs, clientInfo } = req.body || {};
        if (Array.isArray(logs)) {
            store.entries = logs.slice(-300);
            store.updatedAt = new Date().toISOString();
            store.clientInfo = clientInfo || null;
        }
        return res.status(200).json({ ok: true, stored: store.entries.length });
    }

    if (req.method === 'GET') {
        return res.status(200).json(store);
    }

    return res.status(405).json({ error: 'Method not allowed' });
}
