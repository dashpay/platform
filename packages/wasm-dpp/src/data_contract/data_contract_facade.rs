pub struct DataContractFacadeWasm {}
//
// impl DataContractFacade {
//     /**
//      * Create Data Contract
//      *
//      * @param {Identifier|Buffer} ownerId
//      * @param {Object} documents
//      * @return {DataContract}
//      */
//     pub fn create(ownerId, documents) {
//         return this.factory.create(ownerId, documents);
//     }
//
//     /**
//      * Create Data Contract from plain object
//      *
//      * @param {RawDataContract} rawDataContract
//      * @param {Object} options
//      * @param {boolean} [options.skipValidation=false]
//      * @return {Promise<DataContract>}
//      */
//     pub async fn createFromObject(rawDataContract, options = { }) {
//         return this.factory.createFromObject(rawDataContract, options);
//     }
//
//     /**
//      * Create Data Contract from buffer
//      *
//      * @param {Buffer} buffer
//      * @param {Object} options
//      * @param {boolean} [options.skipValidation=false]
//      * @return {Promise<DataContract>}
//      */
//     pub async fn createFromBuffer(buffer, options = { }) {
//         return this.factory.createFromBuffer(buffer, options);
//     }
//
//     /**
//      * Create Data Contract Create State Transition
//      *
//      * @param {DataContract} dataContract
//      * @return {DataContractCreateTransition}
//      */
//     pub fn createDataContractCreateTransition(dataContract) {
//         return this.factory.createDataContractCreateTransition(dataContract);
//     }
//
//     /**
//      * Create Data Contract Update State Transition
//      *
//      * @param {DataContract} dataContract
//      * @return {DataContractUpdateTransition}
//      */
//     pub fn createDataContractUpdateTransition(dataContract) {
//         return this.factory.createDataContractUpdateTransition(dataContract);
//     }
//
//     /**
//      * Validate Data Contract
//      *
//      * @param {DataContract|RawDataContract} dataContract
//      * @return {Promise<ValidationResult>}
//      */
//     pub async fn validate(dataContract) {
//         let rawDataContract;
//         if (dataContract instanceof DataContract) {
//         rawDataContract = dataContract.toObject();
//         } else {
//         rawDataContract = dataContract;
//         }
//
//         return this.validateDataContract(rawDataContract);
//     }
// }