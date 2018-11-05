const dapObjectBaseSchema = require('../../schema/base/dap-object');

/**
 * @param {DapContract} dapContract
 * @return {Object}
 */
module.exports = function enrichDapContractWithBaseDapObject(dapContract) {
  const rawDapContract = dapContract.toJSON();

  const jsonDapContract = JSON.stringify(rawDapContract);
  const clonedDapContract = JSON.parse(jsonDapContract);

  delete clonedDapContract.$schema;

  const { dapObjectsDefinition: clonedDapObjectsDefinition } = clonedDapContract;

  Object.keys(clonedDapObjectsDefinition).forEach((type) => {
    const clonedDapObjectDefinition = clonedDapObjectsDefinition[type];

    const { properties: baseDapObjectProperties } = dapObjectBaseSchema;

    if (!clonedDapObjectDefinition.required) {
      clonedDapObjectDefinition.required = [];
    }

    Object.keys(baseDapObjectProperties).forEach((name) => {
      clonedDapObjectDefinition.properties[name] = baseDapObjectProperties[name];
      clonedDapObjectDefinition.required.push(name);
    });
  });

  return clonedDapContract;
};
