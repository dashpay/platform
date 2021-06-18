import { Platform } from "../../Platform";
import broadcastStateTransition from '../../broadcastStateTransition';
import Document from '@dashevo/dpp/lib/document/Document';
import { signStateTransition } from "../../signStateTransition";

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
    await this.initialize();

    const { dpp } = this;

    const documentsBatchTransition = dpp.document.createStateTransition(documents);

    await signStateTransition(this, documentsBatchTransition, identity);

    // Broadcast state transition also wait for the result to be obtained
    await broadcastStateTransition(this, documentsBatchTransition);

    return documentsBatchTransition;
}
