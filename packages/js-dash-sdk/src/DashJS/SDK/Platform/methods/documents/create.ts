import {Platform} from "../../Platform";

declare interface createOpts {
    [name:string]: any;
}

export function create(this: Platform, typeLocator: string, opts: createOpts): any {
    const contractsName = Object.keys(this.schemas);

    //We can either provide of type `dashpay.profile` or if only one schema provided, of type `profile`.
    const [contractName, fieldType] = (typeLocator.includes('.')) ? typeLocator.split('.') : [contractsName[0], typeLocator];
    this.dpp.setContract(this.schemas[contractName]);

    const document = this.dpp.document.create(fieldType, opts);
    return document;
}

export default create;
