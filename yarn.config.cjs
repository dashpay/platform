module.exports = {
  constraints: async ({ Yarn }) => {
    // Prevent two workspaces from depending on conflicting versions of the same dependency
    for (const dependency of Yarn.dependencies()) {
      if (dependency.type === 'peerDependencies') continue;

      for (const otherDependency of Yarn.dependencies({ ident: dependency.ident })) {
        if (otherDependency.type === 'peerDependencies') continue;

        if (dependency.range !== otherDependency.range) {
          dependency.update(otherDependency.range);
        }
      }
    }

    // Force all workspace dependencies to be made explicit with workspace:*
    for (const workspace of Yarn.workspaces()) {
      for (const dependency of Yarn.dependencies({ workspace })) {
        if (Yarn.workspace({ ident: dependency.ident })) {
          dependency.update('workspace:*');
        }
      }
    }
  },
};
