const _ = require('underscore')

var utils = {
    //Removes rank from mnLists 
    //(rank not needed for Quorum determination and cause mnLists to mutate too frequently)
    getCacheableList: function(mnList) {
        return mnList.filter(l => { return delete l.rank })
    },
    getDiff: function(oldList, newList) {
        return {
            additions: _.filter(newList, i => { return !_.findWhere(oldList, i) }),
            deletions: (_.filter(oldList, i => { return !_.findWhere(newList, i) })).map(mn => mn.vin)
        }
    }

}

module.exports = utils