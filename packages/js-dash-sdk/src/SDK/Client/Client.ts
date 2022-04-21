import { EventEmitter } from 'events';
import { Account, Wallet } from "@dashevo/wallet-lib";
import DAPIClientTransport from "@dashevo/wallet-lib/src/transport/DAPIClientTransport/DAPIClientTransport"
import { Platform } from './Platform';
import { Network } from "@dashevo/dashcore-lib";
import DAPIClient from "@dashevo/dapi-client";
import { contractId as dpnsContractId } from "@dashevo/dpns-contract/lib/systemIds";
import { contractId as dashpayContractId } from "@dashevo/dashpay-contract/lib/systemIds";
import { contractId as masternodeRewardSharesContractId } from "@dashevo/masternode-reward-shares-contract/lib/systemIds";
import { ClientApps, ClientAppsOptions } from "./ClientApps";
import { DashPaySyncWorker } from "./plugins/DashPaySyncWorker/DashPaySyncWorker";
import { DashPay } from "./plugins/DashPay/DashPay";

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
}

/**
 * Client class that wraps all components together to allow integrated payments on both the Dash Network (layer 1)
 * and the Dash Platform (layer 2).
 */
export class Client extends EventEmitter {
    public network: string = 'testnet';
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

        this.network = this.options.network ? this.options.network.toString() : 'testnet';

        // Initialize DAPI Client
        const dapiClientOptions = {
            network: this.network,
        };

        [
            'dapiAddressProvider',
            'dapiAddresses',
            'seeds',
            'timeout',
            'retries',
            'baseBanTime'
        ].forEach((optionName) => {
            if (this.options.hasOwnProperty(optionName)) {
                dapiClientOptions[optionName] = this.options[optionName];
            }
        });

        this.dapiClient = new DAPIClient(dapiClientOptions);

        let dashpayPlugin;
        let dashpaySyncWorker;

        // Initialize a wallet if `wallet` option is preset
        if (this.options.wallet !== undefined) {
            if (this.options.wallet.network !== undefined && this.options.wallet.network !== this.network) {
                throw new Error('Wallet and Client networks are different');
            }

            const transport = new DAPIClientTransport(this.dapiClient);

            const walletOptions = {
              transport,
              network: this.network,
              ...this.options.wallet,
            }

            // If it's a bip44 wallet, we pass the DashPay Worker and DashPay plugin
            if(
              !this.options.wallet.privateKey &&
               this.options.wallet.offlineMode !== true
            ){
              dashpayPlugin = new DashPay();
              dashpaySyncWorker = new DashPaySyncWorker();
              //@ts-ignore
              walletOptions.plugins = [dashpayPlugin, dashpaySyncWorker];
            }

            //@ts-ignore
            this.wallet = new Wallet(walletOptions);

            // @ts-ignore
            this.wallet.on('error', (error, context) => (
                this.emit('error', error, { wallet: context })
            ));
        }

        // @ts-ignore
        this.defaultAccountIndex = this.options.wallet?.defaultAccountIndex || 0;

        this.apps = new ClientApps(Object.assign({
            dpns: {
                contractId: dpnsContractId,
            },
            dashpay: {
                contractId: dashpayContractId,
            },
            masternodeRewardShares: {
                contractId: masternodeRewardSharesContractId,
            }
        }, this.options.apps));

        this.platform = new Platform({
            client: this,
            network: this.network,
            driveProtocolVersion: this.options.driveProtocolVersion,
        });

        if(dashpaySyncWorker && dashpayPlugin){
          dashpayPlugin.inject('platform', this.platform, true)
          dashpaySyncWorker.inject('platform', this.platform, true)
        }
    }

    /**
     * Get Wallet account
     *
     * @param {Account.Options} [options]
     * @returns {Promise<Account>}
     */
    async getWalletAccount(options: Account.Options = {}) : Promise<Account> {
        const { wallet } = this;
        if (!wallet) {
            throw new Error('Wallet is not initialized, pass `wallet` option to Client');
        }

        options = {
            index: this.defaultAccountIndex,
            ...options,
        }
        const account = await wallet.getAccount(options);

        try {
          const dashpayworker = account.getWorker('DashPaySyncWorker');
          //@ts-ignore
          await dashpayworker.execute();
        } catch {}

        return account;
    }

    /**
     * disconnect wallet from Dapi
     * @returns {void}
     */
    async disconnect() {
        if (this.wallet) {
            await this.wallet.disconnect();
        }
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
