import {Platform} from "../../Platform";

/**
 * Create and prepare contracts for the platform
 *
 * @param {Platform} this - bound instance class
 * @param documentDefinitions - document definitions
 * @param identity - identity
 * @returns created contracts
 */
export function create(this: Platform, documentDefinitions: any, identity: any): Promise<any> {
    return this.dpp.dataContract.create(identity.getId(), documentDefinitions);;
}

export default create;
