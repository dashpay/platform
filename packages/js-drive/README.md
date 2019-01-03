# DashDrive

[![Build Status](https://travis-ci.com/dashevo/dashdrive.svg?token=Pzix7aqnMuGS9c6BmBz2&branch=master)](https://travis-ci.com/dashevo/dashdrive)

> Dash network data storage backend service.

An [event-sourced](https://martinfowler.com/eaaDev/EventSourcing.html) metadata service built around the Command-Query Responsibility Segregation ([CQRS](https://martinfowler.com/bliki/CQRS.html)) pattern.

## Table of Contents
- [Install](#install)
- [Usage](#usage)
- [Configuration](#configuration)
- [DashDrive API](#dashdrive-api)
- [Tests](#tests)
- [Maintainer](#maintainer)
- [Contributing](#contributing)
- [License](#license)

## Install

1. [Install Node.JS 8.10.0 and higher](https://nodejs.org/en/download/)
2. [Install Docker](https://docs.docker.com/install/)
3. [Install Docker compose](https://docs.docker.com/compose/install/)
4. Copy `.env.example` to `.env` file
5. Install npm dependencies: `npm install`

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

## Configuration

DashDrive uses environment variables for configuration.
Variables are read from `.env` file and can be overwritten by variables
defined in env or directly passed to the process.

See all available settings in [.env.example](.env.example).

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

#### removeSTPacket

Remove State Transition Packet from DashDrive storage

##### Params

| name       | type   | description                                  |
|------------|--------|----------------------------------------------|
| packetHash | string | ST Packet hash previously submitted to Drive |

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

#### getSyncInfo

Get Sync information

##### Params

None

##### Response

| name    | type   | description     |
|---------|--------|-----------------|
| result  | object | SyncInfo object |

### Errors

| code | message                   | meaning                                                                  |
|------|---------------------------|--------------------------------------------------------------------------|
| 100  | Initial sync in progress  | DashDrive responds with error until initial sync process is not complete |

Standard errors listed in [JSON-RPC specification](https://www.jsonrpc.org/specification).

## Tests

[Read](test/) about tests in `test/` folder. 

## Maintainer

[@shumkov](https://github.com/shumkov)

## Contributing

Feel free to dive in! [Open an issue](https://github.com/dashevo/dashdrive/issues/new) or submit PRs.

## License

[MIT](LICENSE) &copy; Dash Core Group, Inc.
