import crypto from 'crypto';

import Client from '../Client';
import { IStateTransitionResult } from './IStateTransitionResult';
import { WasmPlatformAdapter } from './adapters/WasmPlatformAdapter';

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
  enablePlatform?: boolean,
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
  // WASM SDK adapter for platform operations
  private adapter?: WasmPlatformAdapter;
  // Direct access to wasm-sdk instance for method delegation
  public wasmSdk?: any;
  
  // Legacy DPP - will be removed once migration is complete
  dpp?: any;
  
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

  private platformEnabled: boolean;

  /**
   * Construct some instance of Platform
   *
   * @param {PlatformOpts} options - options for Platform
   */
  constructor(options: PlatformOpts) {
    // Platform functionality can be disabled for core-only builds
    this.platformEnabled = options.enablePlatform !== false;
    
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
    
    // Initialize adapter if platform is enabled
    if (this.platformEnabled) {
      this.adapter = new WasmPlatformAdapter(
        this.client.getDAPIClient(),
        this.client.network,
        true // proofs enabled by default
      );
      // Set the platform reference in the adapter
      this.adapter.setPlatform(this);
    }
  }

  async initialize() {
    if (!this.platformEnabled) {
      throw new Error('Platform functionality is disabled. Use full SDK build or enable platform.');
    }

    if (!this.wasmSdk && this.adapter) {
      try {
        await this.adapter.initialize();
        this.wasmSdk = await this.adapter.getSdk();
        
        // Set protocol version if not already set
        if (this.protocolVersion === undefined) {
          const mappedProtocolVersion = Platform.networkToProtocolVersion.get(
            this.client.network,
          );

          this.protocolVersion = mappedProtocolVersion !== undefined
            ? mappedProtocolVersion : 1; // Default to 1
        }
      } catch (error) {
        this.logger.error('Failed to initialize wasm-sdk:', error);
        throw error;
      }
    }
  }

  /**
   * Get the platform adapter for advanced usage
   */
  getAdapter(): WasmPlatformAdapter | undefined {
    return this.adapter;
  }

  /**
   * Dispose of platform resources
   */
  async dispose(): Promise<void> {
    if (this.adapter) {
      await this.adapter.dispose();
    }
    this.wasmSdk = undefined;
  }

  /**
   * Legacy method for backward compatibility during migration
   * Will be removed once migration is complete
   */
  static async initializeDppModule(): Promise<any> {
    // This is now a no-op as we use wasm-sdk
    console.warn('Platform.initializeDppModule() is deprecated. Platform now uses wasm-sdk.');
    return {};
  }
}