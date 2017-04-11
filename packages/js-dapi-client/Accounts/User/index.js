const User = function(){
    let self = this;
    return {
        create:require('./create').create.call(self),
        login:require('./login').login.call(self),
        search:require('./search').search.call(self),
        send:require('./send').send.call(self),
        update:require('./update').update.call(self),
        remove:require('./remove').remove.call(self),
    }
};

exports.User = User;