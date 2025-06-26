import loadWasmDpp, { DashPlatformProtocol, getLatestProtocolVersion } from '@dashevo/wasm-dpp';
import type { DPPModule } from '@dashevo/wasm-dpp';
import crypto from 'crypto';

import Client from '../Client';
import { IStateTransitionResult } from './IStateTransitionResult';

import createAssetLockTransaction from './createAssetLockTransaction';

import broadcastDocument from './methods/documents/broadcast';
import createDocument from './methods/documents/create';
import getDocument from './methods/documents/get';

import publishContract from './methods/contracts/publish';
import updateContract from './methods/contracts/update';
import createContract from './methods/contracts/create';
import getContract from './methods/contracts/get';
import getContractHistory from './methods/contracts/history';

import getIdentity from './methods/identities/get';
import registerIdentity from './methods/identities/register';
import topUpIdentity from './methods/identities/topUp';
import creditTransferIdentity from './methods/identities/creditTransfer';
import creditWithdrawal from './methods/identities/creditWithdrawal';
import updateIdentity from './methods/identities/update';
import createIdentityCreateTransition from './methods/identities/internal/createIdentityCreateTransition';
import createIdentityTopUpTransition from './methods/identities/internal/createIdentityTopUpTransition';
import createAssetLockProof from './methods/identities/internal/createAssetLockProof';
import waitForCoreChainLockedHeight from './methods/identities/internal/waitForCoreChainLockedHeight';

import registerName from './methods/names/register';
import resolveName from './methods/names/resolve';
import resolveNameByRecord from './methods/names/resolveByRecord';
import searchName from './methods/names/search';
import broadcastStateTransition from './broadcastStateTransition';

import logger, { ConfigurableLogger } from '../../../logger';
import Fetcher from './Fetcher';
import NonceManager from './NonceManager/NonceManager';

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
  creditTransfer: Function,
  withdrawCredits: Function,
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
  history: Function,
}

/**
 * Class for Dash Platform
 *
 * @param documents - documents
 * @param identities - identities
 * @param names - names
 * @param contracts - contracts
 */
export class Platform {
  // TODO: Address in further type system improvements
  //  Do we want to refactor all methods to check
  //  whether dpp is initialized instead of ts-ignoring?
  // @ts-ignore
  dpp: DashPlatformProtocol;

  protocolVersion?: number;

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

  public logger: ConfigurableLogger;

  /**
     * Broadcasts state transition
     * @param {Object} stateTransition
     */
  public broadcastStateTransition(stateTransition: any): Promise<IStateTransitionResult | void> {
    return broadcastStateTransition(this, stateTransition);
  }

  client: Client;

  private static readonly networkToProtocolVersion: Map<string, number> = new Map([
    ['testnet', 1],
  ]);

  protected fetcher: Fetcher;

  public nonceManager: NonceManager;

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
      history: getContractHistory.bind(this),
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
      creditTransfer: creditTransferIdentity.bind(this),
      update: updateIdentity.bind(this),
      withdrawCredits: creditWithdrawal.bind(this),
      utils: {
        createAssetLockProof: createAssetLockProof.bind(this),
        createAssetLockTransaction: createAssetLockTransaction.bind(this),
        createIdentityCreateTransition: createIdentityCreateTransition.bind(this),
        createIdentityTopUpTransition: createIdentityTopUpTransition.bind(this),
        waitForCoreChainLockedHeight: waitForCoreChainLockedHeight.bind(this),
      },
    };

    this.client = options.client;
    const walletId = this.client.wallet ? this.client.wallet.walletId : 'noid';
    this.logger = logger.getForId(walletId);

    // use protocol version from options if set
    if (options.driveProtocolVersion !== undefined) {
      this.protocolVersion = options.driveProtocolVersion;
    }

    this.fetcher = new Fetcher(this.client.getDAPIClient());
    this.nonceManager = new NonceManager(this.client.getDAPIClient());
  }

  async initialize() {
    if (!this.dpp) {
      await Platform.initializeDppModule();

      if (this.protocolVersion === undefined) {
        // use mapped protocol version otherwise
        // fallback to one that set in dpp as the last option

        const mappedProtocolVersion = Platform.networkToProtocolVersion.get(
          this.client.network,
        );

        this.protocolVersion = mappedProtocolVersion !== undefined
          ? mappedProtocolVersion : getLatestProtocolVersion();
      }

      // eslint-disable-next-line

      this.dpp = new DashPlatformProtocol(
        {
          generate: () => crypto.randomBytes(32),
        },
        this.protocolVersion,
      );
    }
  }

  // Explicitly provide DPPModule as return type.
  // If we don't do it, typescript behaves weird and in compiled Platform.d.ts
  // this code looks like this.
  //
  // ```
  // static initializeDppModule(): Promise<typeof import("@dashevo/wasm-dppdist/dpp")>;
  // ```
  //
  // Slash is missing before `dist` and TS compilation in consumers is breaking
  static async initializeDppModule(): Promise<DPPModule> {
    return loadWasmDpp();
  }
}
