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
npm run sync
```

### Start API process

```bash
npm run api
```

## DashDrive API

DashDrive provides [JSON-RPC 2.0](https://www.jsonrpc.org/specification) API for interaction with data.

### RPC methods

#### addSTPacket

Add State Transition Packet to DashDrive storage

##### Params

| name    | type   | description                            |
|---------|--------|----------------------------------------|
| packet  | string | ST Packet object serialized using CBOR |

##### Response

| name    | type   | description                                  |
|---------|--------|----------------------------------------------|
| result  | string | ST Packet [CID](https://github.com/ipld/cid) |

#### fetchDapContact

Fetch DAP Contract from DashDrive State View

##### Params

| name    | type   | description  |
|---------|--------|--------------|
| dapId   | string | DAP ID       |

##### Response

| name    | type   | description         |
|---------|--------|---------------------|
| result  | object | DAP Contact object  |

#### fetchDapObjects

Fetch DAP Objects from DashDrive State View

##### Params

| name    | type   | description          |
|---------|--------|----------------------|
| dapId   | string | DAP ID               |
| type    | string | DAP Objects type     |
| options | object | Options              |

Fetch method options:

| name       | type   | description                                                                             |
|------------|--------|-----------------------------------------------------------------------------------------|
| where      | object | [MongoDB query](https://docs.mongodb.com/manual/reference/operator/query/)              |
| orderBy    | object | [MongoDB sort](https://docs.mongodb.com/manual/reference/method/cursor.sort/)           |
| limit      | number | [MongoDB limit](https://docs.mongodb.com/manual/reference/method/cursor.limit/)         |
| startAt    | number | [MongoDB skip](https://docs.mongodb.com/manual/reference/method/cursor.skip/)           |
| startAfter | number | Exclusive [MongoDB skip](https://docs.mongodb.com/manual/reference/method/cursor.skip/) |

##### Response

| name    | type   | description  |
|---------|--------|--------------|
| result  | array  | DAP objects  |

### Errors

| code | message                   | meaning                                                                  |
|------|---------------------------|--------------------------------------------------------------------------|
| 100  | Initial sync in progress  | DashDrive responds with error until initial sync process is not complete |

Standard errors listed in [JSON-RPC specification](https://www.jsonrpc.org/specification).

## Tests

[Read](test/) about tests in `test/` folder. 
