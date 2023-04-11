import { Identifier } from '@dashevo/wasm-dpp';

/**
 * Interface for ClientApps
 */
export interface ClientAppsOptions {
  [name: string]: ClientAppDefinitionOptions,
}

interface ClientAppDefinitionOptions {
  contractId: Identifier | string,
  contract?: any
}

interface ClientAppDefinition {
  contractId: Identifier,
  contract?: any
}

type ClientAppsList = Record<string, ClientAppDefinition>;

export class ClientApps {
  private apps: ClientAppsList = {};

  constructor(apps: ClientAppsOptions = {}) {
    Object.entries(apps).forEach(([name, definition]) => this.set(name, definition));
  }

  /**
     * Set app
     *
     * @param {string} name
     * @param {ClientAppDefinitionOptions} options
     */
  set(name: string, options: ClientAppDefinitionOptions) {
    this.apps[name] = {
      contractId: Identifier.from(options.contractId),
      contract: options.contract,
    };
  }

  /**
     * Get app definition by name
     *
     * @param {string} name
     * @return {ClientAppDefinition}
     */
  get(name: string): ClientAppDefinition {
    if (!this.has(name)) {
      throw new Error(`Application with name ${name} is not defined`);
    }

    return this.apps[name];
  }

  /**
     * Check if app is defined
     *
     * @param {string} name
     * @return {boolean}
     */
  has(name: string): boolean {
    return Boolean(this.apps[name]);
  }

  /**
     * Get all apps
     *
     * @return {ClientAppsList}
     */
  getNames(): Array<string> {
    return Object.keys(this.apps);
  }
}
