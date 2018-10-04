# Dash Network end-to-end tests

## Introduction
This repository allows to run end-to-end tests against Dash Evolution Network.

## Pre-requisites
A testnet or devnet should be running. If not you can deploy your own network with [dash-network-deploy](https://github.com/dashpay/dash-network-deploy).

## Configuration
Configure DAPI Client seeds and port in `.env` file.
Use [.env.example](https://github.com/dashpay/dash-network-e2e-tests/blob/master/.env.example) as example.

## Run the tests
1. Install [Node.JS](https://nodejs.org/en/download/) and dependencies:
```bash
npm install
```

2. Run tests:
```bash
npm test
```
