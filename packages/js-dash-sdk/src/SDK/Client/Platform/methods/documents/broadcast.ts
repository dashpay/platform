import { ExtendedDocument } from '@dashevo/wasm-dpp';
import { Platform } from '../../Platform';
import broadcastStateTransition from '../../broadcastStateTransition';
import { signStateTransition } from '../../signStateTransition';

/**
 * Broadcast document onto the platform
 *
 * @param {Platform} this - bound instance class
 * @param {Object} documents
 * @param {ExtendedDocument[]} [documents.create]
 * @param {ExtendedDocument[]} [documents.replace]
 * @param {ExtendedDocument[]} [documents.delete]
 * @param identity - identity
 */
export default async function broadcast(
  this: Platform,
  documents: {
    create?: ExtendedDocument[],
    replace?: ExtendedDocument[],
    delete?: ExtendedDocument[]
  },
  identity: any,
): Promise<any> {
  this.logger.debug('[Document#broadcast] Broadcast documents', {
    create: documents.create?.length || 0,
    replace: documents.replace?.length || 0,
    delete: documents.delete?.length || 0,
  });
  await this.initialize();

  // If wasm-sdk is available, delegate to it
  if (this.wasmSdk && this.getAdapter()) {
    const adapter = this.getAdapter()!;
    
    // Get identity private key for signing
    const account = await this.client.getWalletAccount();
    
    // Get the key for document operations (index 1)
    const { privateKey: documentPrivateKey } = account.identities
      .getIdentityHDKeyById(identity.getId().toString(), 1);
    
    // Convert private key to WIF format
    const privateKeyWIF = adapter.convertPrivateKeyToWIF(documentPrivateKey);
    
    // Convert identity to hex format
    const identityHex = identity.toBuffer().toString('hex');
    
    // Process documents for each operation type
    const results: any[] = [];
    
    // Handle document creation/replacement
    if (documents.create || documents.replace) {
      const documentsToUpsert = [
        ...(documents.create || []),
        ...(documents.replace || [])
      ];
      
      if (documentsToUpsert.length > 0) {
        // Group by data contract and document type
        const groupedDocs = new Map<string, ExtendedDocument[]>();
        
        for (const doc of documentsToUpsert) {
          const key = `${doc.getDataContractId().toString()}_${doc.getType()}`;
          if (!groupedDocs.has(key)) {
            groupedDocs.set(key, []);
          }
          groupedDocs.get(key)!.push(doc);
        }
        
        // Process each group
        for (const [key, docs] of groupedDocs) {
          const [dataContractId, documentType] = key.split('_');
          
          // Convert documents to JSON
          const documentsData = docs.map(doc => ({
            id: doc.getId()?.toString(),
            data: doc.getData(),
            createdAt: doc.getCreatedAt(),
            updatedAt: doc.getUpdatedAt(),
          }));
          
          const documentsJson = JSON.stringify(documentsData);
          const putType = documents.replace?.includes(docs[0]) ? 'replace' : 'create';
          
          this.logger.debug(`[Document#broadcast] Calling wasm-sdk documentsPut for ${documentType}`);
          
          const result = await this.wasmSdk.documentsPut(
            dataContractId,
            documentType,
            documentsJson,
            identityHex,
            privateKeyWIF,
            putType
          );
          
          results.push(result);
        }
      }
    }
    
    // Handle document deletion
    if (documents.delete && documents.delete.length > 0) {
      // Group by data contract and document type
      const groupedDeletes = new Map<string, string[]>();
      
      for (const doc of documents.delete) {
        const key = `${doc.getDataContractId().toString()}_${doc.getType()}`;
        if (!groupedDeletes.has(key)) {
          groupedDeletes.set(key, []);
        }
        groupedDeletes.get(key)!.push(doc.getId().toString());
      }
      
      // Process each group
      for (const [key, docIds] of groupedDeletes) {
        const [dataContractId, documentType] = key.split('_');
        
        this.logger.debug(`[Document#broadcast] Calling wasm-sdk documentsDelete for ${documentType}`);
        
        const result = await this.wasmSdk.documentsDelete(
          dataContractId,
          documentType,
          docIds,
          identityHex,
          privateKeyWIF
        );
        
        results.push(result);
      }
    }
    
    // Handle document acknowledgment for cache
    if (documents.create) {
      documents.create.forEach((document) => {
        const documentLocator = `${document.getDataContractId().toString()}/${document.getType()}`;
        this.fetcher.acknowledgeKey(documentLocator);
      });
    }
    
    if (documents.delete) {
      documents.delete.forEach((document) => {
        const documentLocator = `${document.getDataContractId().toString()}/${document.getType()}`;
        this.fetcher.forgetKey(documentLocator);
      });
    }
    
    this.logger.debug('[Document#broadcast] Broadcasted documents via wasm-sdk', {
      results: results.length,
    });
    
    // Return the results
    return results;
  }

  const { dpp } = this;

  const identityId = identity.getId();
  const dataContractId = [
    ...(documents.create || []),
    ...(documents.replace || []),
    ...(documents.delete || []),
  ][0]?.getDataContractId();

  if (!dataContractId) {
    throw new Error('Data contract ID is not found');
  }

  const identityContractNonce = await this.nonceManager
    .bumpIdentityContractNonce(identityId, dataContractId);

  const documentsBatchTransition = dpp.document.createStateTransition(documents, {
    [identityId.toString()]: {
      [dataContractId.toString()]: identityContractNonce.toString(),
    },
  });

  this.logger.silly('[Document#broadcast] Created documents batch transition');

  await signStateTransition(this, documentsBatchTransition, identity, 1);

  // Broadcast state transition also wait for the result to be obtained
  await broadcastStateTransition(this, documentsBatchTransition);

  // Acknowledge documents identifiers to handle retry attempts to mitigate
  // state transition propagation lag
  if (documents.create) {
    documents.create.forEach((document) => {
      const documentLocator = `${document.getDataContractId().toString()}/${document.getType()}`;
      this.fetcher.acknowledgeKey(documentLocator);
    });
  }

  // Forget documents identifiers to not retry on them anymore
  if (documents.delete) {
    documents.delete.forEach((document) => {
      const documentLocator = `${document.getDataContractId().toString()}/${document.getType()}`;
      this.fetcher.forgetKey(documentLocator);
    });
  }

  this.logger.debug('[Document#broadcast] Broadcasted documents', {
    create: documents.create?.length || 0,
    replace: documents.replace?.length || 0,
    delete: documents.delete?.length || 0,
  });

  return documentsBatchTransition;
}
