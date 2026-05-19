import { EventEmitter } from 'events';
import { Account, Wallet } from '@dashevo/wallet-lib';
import DAPIClientTransport from '@dashevo/wallet-lib/src/transport/DAPIClientTransport/DAPIClientTransport.js';
import dashcore from '@dashevo/dashcore-lib';
const { Network } = dashcore;
import type { Network as NetworkType } from '@dashevo/dashcore-lib';
type Network = NetworkType;
import DAPIClient from '@dashevo/dapi-client';
import dpnsSystemIds from '@dashevo/dpns-contract/lib/systemIds.js';
const { contractId: dpnsContractId } = dpnsSystemIds;
import dashpaySystemIds from '@dashevo/dashpay-contract/lib/systemIds.js';
const { contractId: dashpayContractId } = dashpaySystemIds;
import masternodeRewardSharesSystemIds from '@dashevo/masternode-reward-shares-contract/lib/systemIds.js';
const { contractId: masternodeRewardSharesContractId } = masternodeRewardSharesSystemIds;
import withdrawalsSystemIds from '@dashevo/withdrawals-contract/lib/systemIds.js';
const { contractId: withdrawalsContractId } = withdrawalsSystemIds;
import { Platform } from './Platform/index.js';
import { ClientApps, ClientAppsOptions } from './ClientApps.js';

export interface WalletOptions extends Wallet.IWalletOptions {
  defaultAccountIndex?: number;
}

/**
 * Interface Client Options
 *
 * @param {ClientApps?} [apps] - applications
 * @param {WalletOptions} [wallet] - Wallet options
 * @param {DAPIAddressProvider} [dapiAddressProvider] - DAPI Address Provider instance
 * @param {Array<RawDAPIAddress|DAPIAddress|string>} [dapiAddresses] - DAPI addresses
 * @param {string[]|RawDAPIAddress[]} [seeds] - DAPI seeds
 * @param {string|Network} [network=evonet] - Network name
 * @param {number} [timeout=2000]
 * @param {number} [retries=3]
 * @param {number} [baseBanTime=60000]
 */
export interface ClientOpts {
  apps?: ClientAppsOptions,
  wallet?: WalletOptions,
  dapiAddressProvider?: any,
  dapiAddresses?: any[],
  seeds?: any[],
  network?: Network | string,
  timeout?: number,
  retries?: number,
  baseBanTime?: number,
  driveProtocolVersion?: number,
  blockHeadersProviderOptions?: any,
  blockHeadersProvider?: any
}

/**
 * Client class that wraps all components together
 * to allow integrated payments on both the Dash Network (layer 1)
 * and the Dash Platform (layer 2).
 */
export class Client extends EventEmitter {
  public network: string = 'mainnet';

  public wallet: Wallet | undefined;

  public account: Account | undefined;

  public platform: Platform;

  public defaultAccountIndex: number | undefined = 0;

  private readonly dapiClient: DAPIClient;

  private readonly apps: ClientApps;

  private options: ClientOpts;

  /**
     * Construct some instance of SDK Client
     *
     * @param {ClientOpts} [options] - options for SDK Client
     */
  constructor(options: ClientOpts = {}) {
    super();

    this.options = options;

    this.network = this.options.network ? this.options.network.toString() : 'mainnet';

    // Initialize DAPI Client
    const dapiClientOptions = {
      network: this.network,
      loggerOptions: {
        identifier: '',
      },
    };

    [
      'dapiAddressProvider',
      'dapiAddresses',
      'seeds',
      'timeout',
      'retries',
      'baseBanTime',
      'blockHeadersProviderOptions',
      'blockHeadersProvider',
    ].forEach((optionName) => {
      // eslint-disable-next-line
      if (this.options.hasOwnProperty(optionName)) {
        dapiClientOptions[optionName] = this.options[optionName];
      }
    });

    // Initialize a wallet if `wallet` option is preset
    if (this.options.wallet !== undefined) {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const walletOpts = this.options.wallet as any;
      if (walletOpts.network !== undefined
        && walletOpts.network !== this.network) {
        throw new Error('Wallet and Client networks are different');
      }

      this.wallet = new Wallet({
        transport: null,
        network: this.network,
        ...this.options.wallet,
      });

      // @ts-ignore
      this.wallet.on('error', (error, context) => (
        this.emit('error', error, { wallet: context })
      ));
    }

    dapiClientOptions.loggerOptions.identifier = this.wallet ? this.wallet.walletId : 'noid';

    this.dapiClient = new DAPIClient(dapiClientOptions);

    if (this.wallet) {
      this.wallet.transport = new DAPIClientTransport(this.dapiClient);
    }

    this.defaultAccountIndex = this.options.wallet?.defaultAccountIndex || 0;

    this.apps = new ClientApps({
      dpns: {
        contractId: dpnsContractId,
      },
      dashpay: {
        contractId: dashpayContractId,
      },
      masternodeRewardShares: {
        contractId: masternodeRewardSharesContractId,
      },
      withdrawals: {
        contractId: withdrawalsContractId,
      },
      ...this.options.apps,
    });

    this.platform = new Platform({
      client: this,
      network: this.network,
      driveProtocolVersion: this.options.driveProtocolVersion,
    });
  }

  /**
     * Get Wallet account
     *
     * @param {Account.Options} [options]
     * @returns {Promise<Account>}
     */
  async getWalletAccount(options: Account.Options = {}) : Promise<Account> {
    if (!this.wallet) {
      throw new Error('Wallet is not initialized, pass `wallet` option to Client');
    }

    return this.wallet.getAccount({
      index: this.defaultAccountIndex,
      synchronize: true,
      ...options,
    });
  }

  /**
     * disconnect wallet from Dapi
     * @returns {void}
     */
  async disconnect() {
    if (this.wallet) {
      await this.wallet.disconnect();
    }

    await this.dapiClient.disconnect();
  }

  /**
     * Get DAPI Client instance
     *
     * @returns {DAPIClient}
     */
  getDAPIClient() : DAPIClient {
    return this.dapiClient;
  }

  /**
     * fetch list of applications
     *
     * @remarks
     * check if returned value can be null on devnet
     *
     * @returns {ClientApps} applications list
     */
  getApps(): ClientApps {
    return this.apps;
  }
}

export default Client;
