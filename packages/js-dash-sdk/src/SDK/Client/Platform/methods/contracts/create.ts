import {Platform} from "../../Platform";

/**
 * Create and prepare contracts for the platform
 *
 * @param {Platform} this - bound instance class
 * @param contractDefinitions - contract definitions
 * @param identity - identity
 * @returns created contracts
 */
export async function create(this: Platform, contractDefinitions: any, identity: any): Promise<any> {
    await this.initialize();

    return this.dpp.dataContract.create(identity.getId(), contractDefinitions);
}

export default create;
