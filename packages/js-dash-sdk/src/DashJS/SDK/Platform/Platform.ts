// @ts-ignore
import DashPlatformProtocol from "@dashevo/dpp";
import {Mnemonic, Network} from "@dashevo/wallet-lib/src/types/types";
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
        this.documents = {
            broadcast: broadcastDocument.bind(this),
            create: createDocument.bind(this),
            fetch: fetchDocument.bind(this),
        };
        // @ts-ignore
        this.contracts = {
            broadcast: broadcastContract.bind(this),
            create: createContract.bind(this),
            fetch: fetchContract.bind(this),
        };
        // @ts-ignore
        this.identities = {
            register: registerIdentity.bind(this),
            create: createIdentity.bind(this),
            get: getIdentity.bind(this),
            search: searchIdentity.bind(this),
        };
        this.dpp = new DashPlatformProtocol(platformOpts);
        this.client = platformOpts.client;
        this.schemas = platformOpts.schemas;
    }
}
