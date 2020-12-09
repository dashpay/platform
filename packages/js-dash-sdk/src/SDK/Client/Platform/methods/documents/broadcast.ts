import { wait } from '../../../../../utils/wait';
import {Platform} from "../../Platform";
import broadcastStateTransition from '../../broadcastStateTransition';
import Document from '@dashevo/dpp/lib/document/Document';

/**
 * get Document DataContract name
 *
 * @param {Platform} platform
 * @param {Document} document
 *
 */
function getDataContractName(platform: Platform, document: Document): string {
    const appNames = platform.client.getApps().getNames();
    const contractId = document.getDataContractId();
    const dataContractName = appNames.find((name) => {
        const clientAppDefinition = platform.client.getApps().get(name);

        return clientAppDefinition.contractId.equals(contractId);
    });

    if (dataContractName === undefined) {
        // we will never reach this code
        throw new Error('DataContractAppDefinition was not found');
    }

    return dataContractName;
}


/**
 *
 * @param {Platform} platform
 * @param {Object} documents
 * @param {Document[]} [documents.create]
 * @param {Document[]} [documents.replace]
 * @param {Document[]} [documents.delete]
 */
async function waitForPropagation(platform: Platform, documents: { create?: Document[], replace?: Document[], delete?: Document[]}): Promise<void> {
    await wait(6000);

    let fetchedDocuments;

    if (documents.create && documents.create.length > 0) {
        const [document] = documents.create;
        const dataContractName = getDataContractName(platform, document);

        do {
            await wait(1000);

            // @ts-ignore
            fetchedDocuments = await platform.client.platform.documents.get(
              `${dataContractName}.${document.getType()}`,
              { where: [['$id', '==', document.getId()]] },
            );
        } while(fetchedDocuments.length === 0);
    } else if (documents.replace && documents.replace.length > 0) {
        const [document] = documents.replace;
        const dataContractName = getDataContractName(platform, document);

        let revision;

        do {
            await wait(1000);

            // @ts-ignore
            fetchedDocuments = await platform.client.platform.documents.get(
              `${dataContractName}.${document.getType()}`,
              { where: [['$id', '==', document.getId()]] },
            );

            revision = fetchedDocuments[0]?.revision || document.revision;
        } while(revision === document.revision);
    } else if (documents.delete && documents.delete.length > 0) {
        const [document] = documents.delete;
        const dataContractName = getDataContractName(platform, document);

        do {
            await wait(1000);

            // @ts-ignore
            fetchedDocuments = await platform.client.platform.documents.get(
              `${dataContractName}.${document.getType()}`,
              { where: [['$id', '==', document.getId()]] },
            );
        } while(fetchedDocuments.length > 0);
    }
}

/**
 * Broadcast document onto the platform
 *
 * @param {Platform} this - bound instance class
 * @param {Object} documents
 * @param {Document[]} [documents.create]
 * @param {Document[]} [documents.replace]
 * @param {Document[]} [documents.delete]
 * @param identity - identity
 */
export default async function broadcast(this: Platform, documents: { create?: Document[], replace?: Document[], delete?: Document[]}, identity: any): Promise<any> {
    const { dpp } = this;

    const documentsBatchTransition = dpp.document.createStateTransition(documents);

    await broadcastStateTransition(this, documentsBatchTransition, identity);

    // Wait some time for propagation
    await waitForPropagation(this, documents);

    return documentsBatchTransition;
}
