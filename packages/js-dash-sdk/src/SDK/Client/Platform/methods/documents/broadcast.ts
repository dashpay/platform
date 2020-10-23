import {Platform} from "../../Platform";
import broadcastStateTransition from '../../broadcastStateTransition';
import Document from '@dashevo/dpp/lib/document/Document'
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
export default async function broadcast(this: Platform, documents: { create: Document[], replace: Document[], delete: Document[]}, identity: any): Promise<any> {
    const { dpp } = this;

    const documentsBatchTransition = dpp.document.createStateTransition(documents);

    await broadcastStateTransition(this, documentsBatchTransition, identity);

    return documentsBatchTransition;
}
