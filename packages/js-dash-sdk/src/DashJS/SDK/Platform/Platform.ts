// @ts-ignore
import DashPlatformProtocol from "@dashevo/dpp";
import {Mnemonic, Network} from "@dashevo/wallet-lib/src/types";
// @ts-ignore
import DAPIClient from "@dashevo/dapi-client"
import {SDKClients, SDKSchemas} from "../SDK";

import broadcastDocument from "./methods/documents/broadcast";
import createDocument from "./methods/documents/create";
import fetchDocument from "./methods/documents/fetch";

import broadcastContract from "./methods/contracts/broadcast";
import createContract from "./methods/contracts/create";
import fetchContract from "./methods/contracts/fetch";


import createIdentity from "./methods/identities/create";
import getIdentity from "./methods/identities/get";
import registerIdentity from "./methods/identities/register";
import searchIdentity from "./methods/identities/search";

export interface PlatformOpts {
    client: DAPIClient,
    schemas: SDKSchemas
}


export class Platform {
    dpp: DashPlatformProtocol;
    public documents: {
        broadcast:Function,
        create:Function,
        fetch:Function
    };
    public identities: {
        create:Function,
        get:Function,
        register:Function,
        search:Function,
    };
    public contracts: {
        broadcast:Function,
        create:Function,
        fetch:Function
    };
    client: DAPIClient;
    schemas: SDKSchemas;

    constructor(platformOpts: PlatformOpts) {
        // @ts-ignore
        this.documents = {};
        // @ts-ignore
        this.contracts = {};
        // @ts-ignore
        this.identities = {};
        this.dpp = new DashPlatformProtocol(platformOpts);
        this.client = platformOpts.client;
        this.schemas = platformOpts.schemas;
    }
}

Platform.prototype.documents.broadcast = broadcastDocument;
Platform.prototype.documents.create = createDocument;
Platform.prototype.documents.fetch = fetchDocument;

Platform.prototype.contracts.broadcast = broadcastContract;
Platform.prototype.contracts.create = createContract;
Platform.prototype.contracts.fetch = fetchContract;

Platform.prototype.identities.create = createIdentity;
Platform.prototype.identities.get = getIdentity;
Platform.prototype.identities.register = registerIdentity;
Platform.prototype.identities.search = searchIdentity;
