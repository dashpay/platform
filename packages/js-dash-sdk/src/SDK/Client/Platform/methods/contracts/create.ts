import {Platform} from "../../Platform";

/**
 * Create and prepare contracts for the platform
 *
 * @param {Platform} this - bound instance class
 * @param contractDefinitions - contract definitions
 * @param identity - identity
 * @returns created contracts
 */
export function create(this: Platform, contractDefinitions: any, identity: any): Promise<any> {
    return this.dpp.dataContract.create(identity.getId(), contractDefinitions);
}

export default create;
