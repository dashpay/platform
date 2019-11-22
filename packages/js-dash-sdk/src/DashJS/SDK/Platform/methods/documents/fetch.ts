import {Platform} from "../../Platform";

declare interface fetchOpts {
    where: any;
    orderBy: any;
    limit: number;
    startAt: number;
    startAfter: number;
}

export function fetch(this: Platform, typeLocator: string, opts: fetchOpts): any {
    const contractsName = Object.keys(this.schemas);

    //We can either provide of type `dashpay.profile` or if only one schema provided, of type `profile`.
    const [contractName, fieldType] = (typeLocator.includes('.')) ? typeLocator.split('.') : [contractsName[0], typeLocator];
    this.dpp.setContract(this.schemas[contractName]);

    const contractId = this.dpp.getContract().getId();

    return this.client.fetchDocuments(contractId, fieldType, opts);
}

export default fetch;
