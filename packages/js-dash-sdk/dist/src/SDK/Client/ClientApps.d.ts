import Identifier from '@dashevo/dpp/lib/Identifier';
/**
 * Interface for ClientApps
 */
export interface ClientAppsOptions {
    [name: string]: ClientAppDefinitionOptions;
}
interface ClientAppDefinitionOptions {
    contractId: Identifier | string;
    contract?: any;
}
interface ClientAppDefinition {
    contractId: Identifier;
    contract?: any;
}
export declare class ClientApps {
    private apps;
    constructor(apps?: ClientAppsOptions);
    /**
       * Set app
       *
       * @param {string} name
       * @param {ClientAppDefinitionOptions} definition
       */
    set(name: string, definition: ClientAppDefinitionOptions): void;
    /**
       * Get app definition by name
       *
       * @param {string} name
       * @return {ClientAppDefinition}
       */
    get(name: string): ClientAppDefinition;
    /**
       * Check if app is defined
       *
       * @param {string} name
       * @return {boolean}
       */
    has(name: string): boolean;
    /**
       * Get all apps
       *
       * @return {ClientAppsList}
       */
    getNames(): Array<string>;
}
export {};
