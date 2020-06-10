import {Platform} from "../../Platform";

declare interface createOpts {
    [name:string]: any;
}

/**
 * Create and prepare documents for the platform
 *
 * @param {Platform} this - bound instance class
 * @param {string} typeLocator - type locator
 * @param identity - identity
 * @param {Object} [data] - options
 */
export async function create(this: Platform, typeLocator: string, identity: any, data: createOpts = {}): Promise<any> {
    const { dpp } = this;

    const appNames = Object.keys(this.apps);
    //We can either provide of type `dashpay.profile` or if only one schema provided, of type `profile`.
    const [appName, fieldType] = (typeLocator.includes('.')) ? typeLocator.split('.') : [appNames[0], typeLocator];


    if(!this.apps[appName]){
        throw new Error(`Cannot find contractId for ${appName}`);
    }

    const dataContract = await this.contracts.get(this.apps[appName].contractId);

    return dpp.document.create(
        dataContract,
        identity.getId(),
        fieldType,
        data,
    );
}

export default create;
