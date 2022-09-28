// @ts-ignore
import DashPlatformProtocol from "@dashevo/dpp";

import Client from "../Client";
import { IStateTransitionResult } from './IStateTransitionResult';

import createAssetLockTransaction from "./createAssetLockTransaction";

import broadcastDocument from "./methods/documents/broadcast";
import createDocument from "./methods/documents/create";
import getDocument from "./methods/documents/get";

import publishContract from "./methods/contracts/publish";
import updateContract from "./methods/contracts/update";
import createContract from "./methods/contracts/create";
import getContract from "./methods/contracts/get";

import getIdentity from "./methods/identities/get";
import registerIdentity from "./methods/identities/register";
import topUpIdentity from "./methods/identities/topUp";
import updateIdentity from "./methods/identities/update";
import createIdentityCreateTransition from "./methods/identities/internal/createIdentityCreateTransition";
import createIdentityTopUpTransition from "./methods/identities/internal/createIdnetityTopUpTransition";
import createAssetLockProof from "./methods/identities/internal/createAssetLockProof";
import waitForCoreChainLockedHeight from "./methods/identities/internal/waitForCoreChainLockedHeight";

import registerName from "./methods/names/register";
import resolveName from "./methods/names/resolve";
import resolveNameByRecord from "./methods/names/resolveByRecord";
import searchName from "./methods/names/search";
import broadcastStateTransition from "./broadcastStateTransition";
import StateRepository from './StateRepository';
import { latestVersion as latestProtocolVersion } from "@dashevo/dpp/lib/version/protocolVersion";

/**
 * Interface for PlatformOpts
 *
 * @remarks
 * required parameters include { client, apps }
 */
export interface PlatformOpts {
    client: Client,
    network: string,
    driveProtocolVersion?: number,
}

/**
 * @param {Function} broadcast - broadcast records onto the platform
 * @param {Function} create - create records which can be broadcasted
 * @param {Function} get - get records from the platform
 */
interface Records {
    broadcast: Function,
    create: Function,
    get: Function,
}

/**
 * @param {Function} register - register a domain
 * @param {Function} resolve - resolve domain by a name
 * @param {Function} resolveByRecord - resolve domain by it's record
 * @param {Function} search - search domain
 */
interface DomainNames {
    register: Function,
    resolve: Function,
    resolveByRecord: Function,
    search: Function,
}

interface Identities {
    get: Function,
    register: Function,
    topUp: Function,
    update: Function,
    utils: {
      createAssetLockTransaction: Function
      createAssetLockProof: Function
      createIdentityCreateTransition: Function
      createIdentityTopUpTransition: Function
      waitForCoreChainLockedHeight: Function
    }
}

interface DataContracts {
    update: Function,
    publish: Function,
    create: Function,
    get: Function,
}

/**
 * Class for Dash Platform
 *
 * @param documents - documents
 * @param identities - identites
 * @param names - names
 * @param contracts - contracts
 */
export class Platform {
    dpp: DashPlatformProtocol;

    public documents: Records;
    /**
     * @param {Function} get - get identities from the platform
     * @param {Function} register - register identities on the platform
     */
    public identities: Identities;
    /**
     * @param {Function} get - get names from the platform
     * @param {Function} register - register names on the platform
     */
    public names: DomainNames;
    /**
     * @param {Function} get - get contracts from the platform
     * @param {Function} create - create contracts which can be broadcasted
     * @param {Function} register - register contracts on the platform
     */
    public contracts: DataContracts;

    /**
     * Broadcasts state transition
     * @param {Object} stateTransition
     */
    public broadcastStateTransition(stateTransition: any): Promise<IStateTransitionResult|void> {
        return broadcastStateTransition(this, stateTransition);
    };

    client: Client;

    private static readonly networkToProtocolVersion: Map<string, number> = new Map([
        ['testnet', 1],
    ]);

    /**
     * Construct some instance of Platform
     *
     * @param {PlatformOpts} options - options for Platform
     */
    constructor(options: PlatformOpts) {
        this.documents = {
            broadcast: broadcastDocument.bind(this),
            create: createDocument.bind(this),
            get: getDocument.bind(this),
        };
        this.contracts = {
            publish: publishContract.bind(this),
            update: updateContract.bind(this),
            create: createContract.bind(this),
            get: getContract.bind(this),
        };
        this.names = {
            register: registerName.bind(this),
            resolve: resolveName.bind(this),
            resolveByRecord: resolveNameByRecord.bind(this),
            search: searchName.bind(this),
        };
        this.identities = {
            register: registerIdentity.bind(this),
            get: getIdentity.bind(this),
            topUp: topUpIdentity.bind(this),
            update: updateIdentity.bind(this),
            utils: {
                createAssetLockProof: createAssetLockProof.bind(this),
                createAssetLockTransaction: createAssetLockTransaction.bind(this),
                createIdentityCreateTransition: createIdentityCreateTransition.bind(this),
                createIdentityTopUpTransition: createIdentityTopUpTransition.bind(this),
                waitForCoreChainLockedHeight: waitForCoreChainLockedHeight.bind(this),
            }
        };

        this.client = options.client;

        const mappedProtocolVersion = Platform.networkToProtocolVersion.get(
            options.network,
        );

        // use protocol version from options if set
        // use mapped one otherwise
        // fallback to one that set in dpp as the last option
        const driveProtocolVersion = options.driveProtocolVersion !== undefined
          ? options.driveProtocolVersion
          : (mappedProtocolVersion !== undefined ? mappedProtocolVersion : latestProtocolVersion);

        const stateRepository = new StateRepository(this.client);

        this.dpp = new DashPlatformProtocol({
            stateRepository,
            protocolVersion: driveProtocolVersion,
            ...options,
        });
    }

    async initialize() {
        await this.dpp.initialize();
    }
}

