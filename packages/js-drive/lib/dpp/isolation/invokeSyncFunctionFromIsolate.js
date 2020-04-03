/**
 * Invokes method inside the isolate and returns copy of the execution result
 * @param {Context} context - reference to the isolate's context's global object
 * @param {string} objectPath - path of the object on which to invoke method,
 * i.e. 'dpp.stateTransition'
 * @param {string} methodName - method name to invoke.
 * If desirable method is dpp.stateTransition.create,
 * then the value of this argument must be 'create'
 * @param {*[]} args - an array of arguments to pass to the invoked method
 * @param {Object} options - additional option for the isolate
 * @returns {any}
 */
function invokeSyncFunctionFromIsolate(
  context,
  objectPath,
  methodName,
  args,
  options = {},
) {
  const properties = objectPath.split('.');
  const { global: jail } = context;

  let objectReference;

  for (const property of properties) {
    if (!objectReference) {
      objectReference = jail.getSync(property);
    } else {
      // noinspection JSUnusedAssignment
      objectReference = objectReference.getSync(property);
    }
  }

  if (!objectReference) { objectReference = jail; }

  const methodReference = objectReference.getSync(methodName);

  return methodReference.applySync(
    objectReference ? objectReference.derefInto() : null,
    args,
    options,
  );
}

module.exports = invokeSyncFunctionFromIsolate;
