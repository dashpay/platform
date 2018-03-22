# DashDrive

Dash network data storage backend service.

An [event-sourced](https://martinfowler.com/eaaDev/EventSourcing.html) metadata service built around the Command-Query Responsibility Segregation ([CQRS](https://martinfowler.com/bliki/CQRS.html)) pattern.

[![Build Status](https://travis-ci.com/dashevo/dashdrive.svg?token=Pzix7aqnMuGS9c6BmBz2&branch=master)](https://travis-ci.com/dashevo/dashdrive)

## Installation

1. [Install Node.JS 8.10.0 and higher](https://nodejs.org/en/download/)
2. [Install Docker](https://docs.docker.com/install/)
3. [Install Docker compose](https://docs.docker.com/compose/install/)
4. Copy `.env.example` to `.env` file

## Usage

### Start development environment

```bash
docker-compose up -d
```

### Start sync process

```bash
npm run storageSync
```

### Start API

```bash
npm run rpc
```

### Run tests

```
npm test
```
