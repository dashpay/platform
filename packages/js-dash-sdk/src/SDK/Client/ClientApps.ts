import Identifier from "@dashevo/dpp/lib/Identifier";

/**
 * Interface for ClientApps
 */
export interface ClientAppsOptions {
    [name: string]: ClientAppDefinitionOptions,
}

interface ClientAppDefinitionOptions {
    contractId: Identifier|string,
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
     * @param {ClientAppDefinitionOptions} definition
     */
    set(name: string, definition: ClientAppDefinitionOptions) {
        definition.contractId = Identifier.from(definition.contractId);

        this.apps[name] = definition;
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
