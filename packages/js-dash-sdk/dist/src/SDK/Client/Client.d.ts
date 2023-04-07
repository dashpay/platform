/// <reference types="node" />
import { EventEmitter } from 'events';
import { Account, Wallet } from '@dashevo/wallet-lib';
import { Network } from '@dashevo/dashcore-lib';
import DAPIClient from '@dashevo/dapi-client';
import { Platform } from './Platform';
import { ClientApps, ClientAppsOptions } from './ClientApps';
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
    apps?: ClientAppsOptions;
    wallet?: WalletOptions;
    dapiAddressProvider?: any;
    dapiAddresses?: any[];
    seeds?: any[];
    network?: Network | string;
    timeout?: number;
    retries?: number;
    baseBanTime?: number;
    driveProtocolVersion?: number;
}
/**
 * Client class that wraps all components together
 * to allow integrated payments on both the Dash Network (layer 1)
 * and the Dash Platform (layer 2).
 */
export declare class Client extends EventEmitter {
    network: string;
    wallet: Wallet | undefined;
    account: Account | undefined;
    platform: Platform;
    defaultAccountIndex: number | undefined;
    private readonly dapiClient;
    private readonly apps;
    private options;
    /**
       * Construct some instance of SDK Client
       *
       * @param {ClientOpts} [options] - options for SDK Client
       */
    constructor(options?: ClientOpts);
    /**
       * Get Wallet account
       *
       * @param {Account.Options} [options]
       * @returns {Promise<Account>}
       */
    getWalletAccount(options?: Account.Options): Promise<Account>;
    /**
       * disconnect wallet from Dapi
       * @returns {void}
       */
    disconnect(): Promise<void>;
    /**
       * Get DAPI Client instance
       *
       * @returns {DAPIClient}
       */
    getDAPIClient(): DAPIClient;
    /**
       * fetch list of applications
       *
       * @remarks
       * check if returned value can be null on devnet
       *
       * @returns {ClientApps} applications list
       */
    getApps(): ClientApps;
}
export default Client;
