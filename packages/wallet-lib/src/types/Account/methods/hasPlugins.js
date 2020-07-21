const _ = require('lodash');
/**
 * To any Plugins (StandardPlugins, Worker,...) will lookup in account for it's presence.
 *
 * @param {[Plugin]} searchedPlugins - Array of constructor or single plugin constructor
 * @return {{found:Boolean, results:[{name: string}]}} search - results with presents plugin
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
    _.each(['workers', 'standard'], (pluginTypeName) => {
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
