# Running Seyeon Oversight with Docker

This document contains instructions for running the Seyeon Oversight system using Docker and Docker Compose.

## Prerequisites

- [Docker](https://docs.docker.com/get-docker/) installed
- [Docker Compose](https://docs.docker.com/compose/install/) installed

## Configuration

1. Create a `.env` file based on the provided example:

```bash
cp .env.example .env
```

2. Edit the `.env` file and add your credentials and settings:

```bash
nano .env  # or use your preferred text editor
```

### Required environment variables:

- `REDIS_URL`: Redis connection URL (already configured for the container)
- `CRYPTOCOMPARE_API_KEY`: Your CryptoCompare API key
- `RAPIDAPI_KEY`: Your RapidAPI key for the Fear & Greed Index
- `SMTP_FROM_EMAIL`: Source email for sending alerts
- `SMTP_TO_EMAIL`: Destination email for alerts
- `SMTP_CC_EMAILS`: List of CC emails separated by commas
- `SMTP_PASSWORD`: SMTP password or application password (recommended for Gmail)

## Building and Running

### Build and start the containers

```bash
docker-compose up -d --build
```

The `-d` flag runs the containers in detached mode (background).

### Check application logs

```bash
docker logs -f seyeon-oversight
```

### Stop the containers

```bash
docker-compose down
```

### Restart the containers

```bash
docker-compose restart
```
