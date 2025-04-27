<div align="center">
  <h1>세연 Oversight</h1>

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Redis](https://img.shields.io/badge/Redis-6.0%2B-red.svg)](https://redis.io/)
[![Docker](https://img.shields.io/badge/Docker-Supported-2496ED.svg)](https://www.docker.com/)

A crypto-asset surveillance system designed for the safe management of investments, featuring real-time price monitoring and alerts for buying, selling, or holding.

</div>

## Overview

**seyeon-oversight** is a program that leverages real-time data obtained through APIs, incorporating various indicators to inform decision-making for crypto assets.

The Seyeon system is built as a Rust workspace with multiple specialized crates.

The trading engine implements a sophisticated signal generation system based on multiple technical indicators:

- **Moving Averages:** 5, 25, 50, 111, 350-day
- **Bollinger Bands:** For volatility measurement
- **RSI:** For overbought/oversold conditions
- **MACD:** For trend identification
- **Pi Cycle Top Indicator:** For market cycle detection
- **Fear & Greed Index:** Integration for market sentiment analysis

## Getting Started

### Prerequisites

- Rust 1.70+
- Linux environment (recommended)
- API keys for:
  - CryptoCompare
  - RapidAPI (for Fear & Greed Index)

### Installation

```bash
# Clone the repository
git clone https://github.com/your-username/seyeon-oversight.git
cd seyeon-oversight

# Build the project
cargo build --release
```

### Configuration

The project uses environment variables for configuration. A `.env.example` file is provided with the required variables.

```bash
# Copy the example environment file to create your own .env file
cp .env.example .env

# Edit the .env file with your API keys and configuration
nano .env  # or use your preferred text editor
```

## Usage

### Running the Trading Simulator

The simulator allows you to test the trading engine with historical data without sending alerts:

```bash
# Simulate all cryptocurrencies in your portfolio
cargo run --bin oversight -- --simulate

# Simulate a specific cryptocurrency
cargo run --bin oversight -- --simulate --crypto BTC

# Adjust simulation period (default: 365 days)
cargo run --bin oversight -- --simulate --days 500

# Combine parameters
cargo run --bin oversight -- --simulate --crypto ETH --days 180
```

The simulator will output:

- ROI (Return on Investment) percentage
- Final portfolio value
- Total number of trades
- Estimated fees paid
- A comparison table ranking assets by performance

### Running the Monitoring System

The monitoring system continuously analyzes crypto assets and sends alerts:

```bash
# Run the monitoring system in background (recommended for production)
nohup cargo run --release --bin oversight > oversight.log 2>&1 &

# Force an immediate daily report generation
cargo run --bin oversight -- --force-report

# Access log files
tail -f oversight.log
```

### API Key Load Balancing

Seyeon Oversight supports multiple API keys to distribute requests and avoid rate limits:

```
# In your .env file, separate multiple keys with commas
CRYPTOCOMPARE_API_KEY=key1,key2,key3
RAPIDAPI_KEY=key1,key2,key3
```

The system will randomly select a key for each request, reducing the chance of hitting API rate limits.

## Security Implementation

### Operational Security Requirements

1. **Mandatory Environmental Controls:**

   - Secure Linux environment (Kernel ≥6.1)
   - TPM 2.0 or Secure Enclave hardware
   - VPN/I2P/Onion network connectivity enforced during operations via Whonix gateway

2. **Network Protection:**

   - Packet encryption for all external communications
   - Firewall isolation of non-essential ports
   - Continuous IP rotation mechanism through Tor circuits

3. **Wallet Security:**
   - Automatically provided wallets with in-memory encryption

## Legal & Operational Disclaimer

This program and its developers expressly disclaim all liability for:

- Loss of funds due to operational errors
- Security breaches from improper environment configuration
- Regulatory consequences of system misuse

Users assume full responsibility for:

1. Wallet/key management practices
2. Compliance with local financial regulations
3. Maintenance of a secure execution environment
4. Transaction monitoring and audit trails

## License

Apache 2.0 - Comprehensive commercial use rights with patent protection

## Credits

**Core Developers**

- [@Denzy](https://github.com/denzylegacy): Founder
- [@ry-diffusion](https://github.com/ry-diffusion): Architecture
