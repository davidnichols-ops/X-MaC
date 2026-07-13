# GNN Inference Server

An optional HTTP server for running GNN inference remotely. This is **not** used by the macOS app — the app uses CoreML for on-device inference. This server is for development, debugging, and Linux environments where CoreML is unavailable.

## When to Use

- **Development**: Test model changes without re-exporting to CoreML
- **Linux**: Run inference on Linux where CoreML is unavailable
- **Batch processing**: Score large numbers of findings via API
- **Model comparison**: A/B test different model versions

## When NOT to Use

- **Production macOS app**: Use the bundled CoreML model instead (on-device, no network)
- **Privacy-sensitive environments**: The server receives filesystem graph data over HTTP

## Setup

```bash
cd gnn
python -m venv .venv
source .venv/bin/activate
pip install fastapi uvicorn torch torch-geometric numpy

# Start the server
python -m uvicorn server.inference_server:app --host 0.0.0.0 --port 8000
```

## API

### `POST /score`

Score a list of findings for safety.

**Request:**
```json
{
  "nodes": [
    {
      "id": 0,
      "features": {
        "category": "cache",
        "size_bytes": 1048576,
        "depth": 5,
        "extension": ".json"
      }
    }
  ],
  "edges": [
    {"source": 0, "target": 1, "type": "same_directory"}
  ]
}
```

**Response:**
```json
{
  "scores": [
    {
      "node_id": 0,
      "safety_score": 0.95,
      "anomaly_score": 0.05,
      "predicted_category": "cache"
    }
  ],
  "model_version": "0.2.0",
  "inference_time_ms": 12.3
}
```

### `GET /health`

Health check endpoint.

### `GET /model/info`

Get model metadata (version, label map, architecture).

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `XMAC_MODEL_PATH` | `/app/model/xmac_gnn.pt` | Path to the trained PyTorch model |
| `XMAC_LABEL_MAP` | `/app/label_map.json` | Path to the category label map |

## Docker

```dockerfile
FROM python:3.11-slim
WORKDIR /app
COPY . .
RUN pip install fastapi uvicorn torch torch-geometric numpy
CMD ["uvicorn", "server.inference_server:app", "--host", "0.0.0.0", "--port", "8000"]
```

```bash
docker build -t xmac-gnn-server .
docker run -p 8000:8000 xmac-gnn-server
```
