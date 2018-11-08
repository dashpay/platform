describe('DapContract', () => {
  describe('constructor', () => {
    it('should create new Dap Contract');
  });
  describe('#getId', () => {
    it('should calculate Dap Contract ID');
  });
  describe('#getSchemaId', () => {
    it('should return JSON Schema $id');
  });
  describe('#setName', () => {
    it('should set name');
  });
  describe('#getName', () => {
    it('should return name');
  });
  describe('#setVersion', () => {
    it('should set version');
  });
  describe('#getVersion', () => {
    it('should return version');
  });
  describe('#setSchema', () => {
    it('should set $$schema');
  });
  describe('#getSchema', () => {
    it('should return $schema');
  });
  describe('#setDapObjectsDefinition', () => {
    it('should set Dap Objects definition');
  });
  describe('#getDapObjectsDefinition', () => {
    it('should return Dap Objects definition');
  });
  describe('#setDefinitions', () => {
    it('should set definitions');
  });
  describe('#getDefinitions', () => {
    it('should return definitions');
  });
  describe('#setDapObjectSchema', () => {
    it('should set Dap Object schema');
  });
  describe('#isDapObjectDefined', () => {
    it('should return true if Dap Object schema is defined');
    it('should return false if Dap Object schema is not defined');
  });
  describe('#getDapObjectSchema', () => {
    it('should return Dap Object Schema');
  });
  describe('#getDapObjectSchemaRef', () => {
    it('should return schema with $ref to Dap Object schema');
  });
  describe('#toJSON', () => {
    it('should return Dap Contract as plain object');
  });
  describe('#serialize', () => {
    it('should return serialized Dap Contract');
  });
  describe('#hash', () => {
    it('should return Dap Contract hash');
  });
  describe('.fromObject', () => {
    it('should create Dap Contract from plain object');
    it('should throw error if data is not valid');
    it('should set definitions if it is present');
  });
  describe('.fromSerialized', () => {
    it('should create Dap Contract from string');
    it('should create Dap Contract from buffer');
  });
  describe('.setSerializer', () => {
    it('should set serializer');
  });
  describe('.setStructureValidator', () => {
    it('should set structure validator');
  });
});
