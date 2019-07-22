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

    if (Object.prototype.hasOwnProperty.call(rawDocumentMetadata, 'stReference')) {
      this.stReference = rawDocumentMetadata.stReference;
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
   * Get raw reference
   *
   * @returns {RawSTReference}
   */
  getSTReference() {
    return this.stReference;
  }

  /**
   * Get the JSON representation of the meta
   *
   * @returns {RawDocumentMetadata}
   */
  toJSON() {
    const json = {
      userId: this.userId,
    };

    if (this.stReference) {
      json.stReference = this.stReference;
    }

    return json;
  }
}

module.exports = DocumentMetadata;
