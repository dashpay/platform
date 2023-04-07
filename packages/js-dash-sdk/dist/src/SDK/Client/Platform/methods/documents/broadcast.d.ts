import Document from '@dashevo/dpp/lib/document/Document';
import { Platform } from '../../Platform';
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
export default function broadcast(this: Platform, documents: {
    create?: Document[];
    replace?: Document[];
    delete?: Document[];
}, identity: any): Promise<any>;
