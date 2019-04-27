class DocumentMetadata {
  /**
   * Create document meta class
   *
   * @param {RawDocumentMetadata} rawDocumentMetadata
   */
  constructor(rawDocumentMetadata) {
    if (Object.prototype.hasOwnProperty.call(rawDocumentMetadata, 'userId')) {
      this.userId = rawDocumentMetadata.userId;
    }
  }

  /**
   * Get user ID
   *
   * @returns {string}
   */
  getUserId() {
    return this.userId;
  }

  /**
   * Get the JSON representation of the meta
   *
   * @returns {RawDocumentMetadata}
   */
  toJSON() {
    return {
      userId: this.userId,
    };
  }
}

module.exports = DocumentMetadata;
