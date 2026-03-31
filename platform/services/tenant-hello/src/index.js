import express from "express";

const app = express();
const port = Number(process.env.PORT ?? "8080");

app.get("/healthz", (_req, res) => res.status(200).send("ok"));

app.get("/", (_req, res) => {
    res.json({
        ok: true,
        service: "tenant-hello",
        ts: new Date().toISOString(),
    });
});

app.listen(port, () => {
    console.log(`[tenant-hello] listening on :${port}`);
});
