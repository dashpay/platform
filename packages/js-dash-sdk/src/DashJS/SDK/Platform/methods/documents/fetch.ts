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
    // FIXME: we may later want a hashmap of schemas and contract IDs

    if(!this.schemas[contractName]){
        throw new Error(`There is no schema set for ${contractName}`)
    }
    const schema = this.schemas[contractName];

    // FIXME : In JS-DPP 0.10, change altered the way we actually manipulate contract id.
    // Therefore, below code is now invalid

    // console.dir(this.dpp.dataContract.factory.validateDataContract(schema),{depth:null});
    // this.dpp.dataContract.setContract(this.schemas[contractName]);
    // this.dpp.dataContract.setDocumentSchema(this.schemas[contractName]);
    // const contractId = this.dpp.getContract().getId();
    // return this.client.fetchDocuments(contractId, fieldType, opts);
}

export default fetch;
