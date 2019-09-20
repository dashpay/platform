const _ = require('lodash');
/**
 * To any object passed (Transaction, ST,..), will try to sign the message given passed keys.
 * @param searchedPlugins {Array} - Array of constructor or single plugin constructor
 */
module.exports = function hasPlugins(searchedPlugins = []) {
  const search = {
    found: false,
    results: [],
  };
  if (!Array.isArray(searchedPlugins)) {
    return hasPlugins.call(this, [searchedPlugins]);
  }
  const { plugins } = this;
  _.each(searchedPlugins, (searchedPlugin) => {
    const result = {};
    _.each(['workers', 'DPAs', 'standard'], (pluginTypeName) => {
      const pluginType = plugins[pluginTypeName];
      _.each(pluginType, (plugin) => {
        if (searchedPlugin.name === plugin.constructor.name) {
          result.name = plugin.name;
        }
      });
    });
    if (result.name) {
      search.results.push(result);
    }
  });
  if (searchedPlugins.length === search.results.length) {
    search.found = true;
  }
  return search;
};
