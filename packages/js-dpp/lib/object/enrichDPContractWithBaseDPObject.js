const dpObjectBaseSchema = require('../../schema/base/dp-object');

/**
 * @typedef {enrichDPContractWithBaseDPObject}
 * @param {DPContract} dpContract
 * @return {Object}
 */
function enrichDPContractWithBaseDPObject(dpContract) {
  const rawDPContract = dpContract.toJSON();

  const jsonDPContract = JSON.stringify(rawDPContract);
  const clonedDPContract = JSON.parse(jsonDPContract);

  delete clonedDPContract.$schema;

  const { dpObjectsDefinition: clonedDPObjectsDefinition } = clonedDPContract;

  Object.keys(clonedDPObjectsDefinition).forEach((type) => {
    const clonedDPObjectDefinition = clonedDPObjectsDefinition[type];

    const { properties: baseDPObjectProperties } = dpObjectBaseSchema;

    if (!clonedDPObjectDefinition.required) {
      clonedDPObjectDefinition.required = [];
    }

    Object.keys(baseDPObjectProperties).forEach((name) => {
      clonedDPObjectDefinition.properties[name] = baseDPObjectProperties[name];
      clonedDPObjectDefinition.required.push(name);
    });
  });

  return clonedDPContract;
}

module.exports = enrichDPContractWithBaseDPObject;
