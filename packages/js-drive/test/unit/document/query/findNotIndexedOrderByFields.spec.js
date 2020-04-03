const findNotIndexedOrderByFields = require('../../../../lib/document/query/findNotIndexedOrderByFields');

describe('findNotIndexedOrderByFields', () => {
  let indexedFields;

  beforeEach(() => {
    indexedFields = [
      [{ $ownerId: 'asc' }, { firstName: 'desc' }],
      [{ $ownerId: 'asc' }, { lastName: 'desc' }, { secondName: 'asc' }, { middleName: 'asc' }],
      [{ $id: 'asc' }],
      [{ $id: 'desc' }],
      [{ address: 'desc' }],
      [{ street: 'desc' }],
      [{ 'arrayWithObjects.item': 'desc' }],
      [{ 'arrayWithObjects.flag': 'desc' }],
      [{ address: 'asc' }, { 'arrayWithObjects.flag': 'desc' }],
      [{ 'arrayWithObjects.flag': 'desc' }, { street: 'asc' }],
      [{ 'arrayWithObjects.country': 'desc' }, { 'arrayWithObjects.language': 'asc' }],
    ];
  });

  it('should pass system $id field', () => {
    const orderByCondition = [
      ['$id', 'desc'],
    ];
    const whereCondition = [];

    const result = findNotIndexedOrderByFields(indexedFields, orderByCondition, whereCondition);

    expect(result).to.be.an('array');
    expect(result).to.be.empty();
  });

  it('should pass with first field on index and empty where', () => {
    const orderByCondition = [
      ['address', 'asc'],
    ];
    const whereCondition = [];

    const result = findNotIndexedOrderByFields(indexedFields, orderByCondition, whereCondition);

    expect(result).to.be.an('array');
    expect(result).to.be.empty();
  });

  it('should pass with first field on index and where contains that field', () => {
    const orderByCondition = [
      ['address', 'asc'],
    ];
    const whereCondition = [
      ['address', '==', 'myAddress'],
    ];

    const result = findNotIndexedOrderByFields(indexedFields, orderByCondition, whereCondition);

    expect(result).to.be.an('array');
    expect(result).to.be.empty();
  });

  it('should pass with second field on index and where contains first field', () => {
    const orderByCondition = [
      ['firstName', 'desc'],
    ];
    const whereCondition = [
      ['$ownerId', '==', 123],
    ];

    const result = findNotIndexedOrderByFields(indexedFields, orderByCondition, whereCondition);

    expect(result).to.be.an('array');
    expect(result).to.be.empty();
  });

  it('should order by two fields with where condition', () => {
    const orderByCondition = [
      ['$ownerId', 'asc'],
      ['firstName', 'desc'],
    ];
    const whereCondition = [['$ownerId', '==', 123]];

    const result = findNotIndexedOrderByFields(indexedFields, orderByCondition, whereCondition);

    expect(result).to.be.an('array');
    expect(result).to.be.empty();
  });

  it('should pass order by two fields with empty where', () => {
    const orderByCondition = [
      ['$ownerId', 'asc'],
      ['firstName', 'desc'],
    ];
    const whereCondition = [];

    const result = findNotIndexedOrderByFields(indexedFields, orderByCondition, whereCondition);

    expect(result).to.be.an('array');
    expect(result).to.be.empty();
  });

  it('should fail on sort by different indices', async () => {
    const orderByCondition = [
      ['address', 'asc'],
      ['$ownerId', 'asc'],
    ];
    const whereCondition = [];

    const result = findNotIndexedOrderByFields(indexedFields, orderByCondition, whereCondition);

    expect(result).to.be.an('array');
    expect(result).to.have.lengthOf(2);
    expect(result).to.deep.members(['address', '$ownerId']);
  });

  it('should order by single field index in two directions', () => {
    let orderByCondition = [
      ['street', 'asc'],
    ];
    const whereCondition = [];

    let result = findNotIndexedOrderByFields(indexedFields, orderByCondition, whereCondition);

    expect(result).to.be.an('array');
    expect(result).to.be.empty();

    orderByCondition = [
      ['street', 'desc'],
    ];

    result = findNotIndexedOrderByFields(indexedFields, orderByCondition, whereCondition);

    expect(result).to.be.an('array');
    expect(result).to.be.empty();
  });

  it('should fail on sort by wrong direction of compound key', async () => {
    const orderByCondition = [
      ['firstName', 'asc'],
      ['$ownerId', 'asc'],
    ];
    const whereCondition = [];

    const result = findNotIndexedOrderByFields(indexedFields, orderByCondition, whereCondition);

    expect(result).to.be.an('array');
    expect(result).to.have.lengthOf(2);
    expect(result).to.deep.members(['firstName', '$ownerId']);
  });

  it('should fail on order by two fields with wrong direction with where condition', () => {
    const orderByCondition = [['firstName', 'desc'], ['$ownerId', 'desc']];
    const whereCondition = [['$ownerId', '==', 123]];

    const result = findNotIndexedOrderByFields(indexedFields, orderByCondition, whereCondition);

    expect(result).to.be.an('array');
    expect(result).to.have.lengthOf(2);
    expect(result).to.deep.members(['firstName', '$ownerId']);
  });

  it('should order by second index key with elementMatch where condition ', () => {
    const orderByCondition = [
      ['street', 'asc'],
    ];
    const whereCondition = [
      ['arrayWithObjects', 'elementMatch', [
        ['flag', '==', true],
      ],
      ]];

    const result = findNotIndexedOrderByFields(indexedFields, orderByCondition, whereCondition);

    expect(result).to.be.an('array');
    expect(result).to.be.empty();
  });

  it('should fail when sorting fields order is not equal their indexed order ', () => {
    const orderByCondition = [
      ['firstName', 'desc'],
      ['$ownerId', 'asc'],
    ];
    const whereCondition = [];

    const result = findNotIndexedOrderByFields(indexedFields, orderByCondition, whereCondition);

    expect(result).to.be.an('array');
    expect(result).to.have.lengthOf(1);
    expect(result).to.deep.members(['firstName']);
  });

  it('should fail when sorting fields order is not equal their indexed order with where condition', () => {
    const orderByCondition = [
      ['middleName', 'asc'],
      ['secondName', 'asc'],
    ];
    const whereCondition = [
      ['lastName', '==', 'Marsh'],
      ['$ownerId', '==', 123],
    ];

    const result = findNotIndexedOrderByFields(indexedFields, orderByCondition, whereCondition);

    expect(result).to.be.an('array');
    expect(result).to.have.lengthOf(1);
    expect(result).to.deep.members(['middleName']);
  });
});
