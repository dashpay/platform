% Prevent two workspaces from depending on conflicting versions of a same dependency

gen_enforced_dependency(WorkspaceCwd, DependencyIdent, DependencyRange2, DependencyType) :-
  workspace_has_dependency(WorkspaceCwd, DependencyIdent, DependencyRange, DependencyType),
  workspace_has_dependency(OtherWorkspaceCwd, DependencyIdent, DependencyRange2, DependencyType2),
  DependencyRange \= DependencyRange2.

% Force all workspace dependencies to be made explicit

gen_enforced_dependency(WorkspaceCwd, DependencyIdent, 'workspace:*', DependencyType) :-
  workspace_ident(_, DependencyIdent),
  workspace_has_dependency(WorkspaceCwd, DependencyIdent, _, DependencyType).

% Ensure workspace versions stay in sync with the root release cadence
%   - Three workspaces must always lead the root major by a fixed offset (M+1, M+3, M+7)
%   - All other workspaces mirror the root version exactly

special_workspace_offset('@dashevo/dash-spv', 1).
special_workspace_offset('dash', 3).
special_workspace_offset('@dashevo/wallet-lib', 7).

% Yarn exposes workspaces by identifier; capture the root version once so the
% dependent rules don't recompute it or rely on hard-coded paths.
root_workspace_version(RootVersion) :-
  workspace_ident(RootWorkspaceCwd, '@dashevo/platform'),
  workspace_field(RootWorkspaceCwd, 'version', RootVersion).

% Workspace metadata can arrive as strings, atoms, or lists of character codes.
% Normalize everything to a list of ASCII codes so arithmetic helpers can work.
normalize_version_list(VersionValue, VersionList) :-
  ( VersionValue = [] ->
      VersionList = []
  ; is_list(VersionValue) ->
      VersionList = VersionValue
  ; atom(VersionValue) ->
      atom_codes(VersionValue, VersionList)
  ; VersionList = VersionValue
  ).

digit_value(48, 0).
digit_value(49, 1).
digit_value(50, 2).
digit_value(51, 3).
digit_value(52, 4).
digit_value(53, 5).
digit_value(54, 6).
digit_value(55, 7).
digit_value(56, 8).
digit_value(57, 9).

is_dot(46).

split_major_rest(VersionList, MajorNumber, Rest) :-
  split_major_rest_loop(VersionList, 0, MajorNumber, Rest).

split_major_rest_loop([Dot|Rest], Acc, Acc, [Dot|Rest]) :-
  is_dot(Dot).
split_major_rest_loop([Digit|Tail], Acc0, Major, Rest) :-
  digit_value(Digit, Value),
  Acc1 is Acc0 * 10 + Value,
  split_major_rest_loop(Tail, Acc1, Major, Rest).

% Convert a non-negative integer back into its ASCII code sequence. We avoid
% relying on Prolog built-ins that aren't available in the Teal interpreter
% shipped with Yarn.
number_to_digit_codes(Number, Codes) :-
  Number >= 0,
  ( Number < 10 ->
      DigitCode is Number + 48,
      Codes = [DigitCode]
  ;   Quotient is Number // 10,
      Remainder is Number mod 10,
      number_to_digit_codes(Quotient, Prefix),
      DigitCode is Remainder + 48,
      append(Prefix, [DigitCode], Codes)
  ).

version_with_offset(RootVersion, Offset, TargetVersion) :-
  normalize_version_list(RootVersion, RootVersionList),
  % Split the root version into the leading major (as an integer) and the
  % untouched suffix (<minor>.<patch>[-prerelease]).
  split_major_rest(RootVersionList, RootMajorNumber, Rest),
  TargetMajorNumber is RootMajorNumber + Offset,
  number_to_digit_codes(TargetMajorNumber, TargetDigits),
  % Recombine the bumped major with the original suffix and return it as a
  % string atom so Yarn can compare it with package.json fields.
  append(TargetDigits, Rest, TargetVersionList),
  atom_codes(TargetVersion, TargetVersionList).

gen_enforced_field(WorkspaceCwd, 'version', ExpectedVersion) :-
  workspace_ident(WorkspaceCwd, Ident),
  special_workspace_offset(Ident, Offset),
  root_workspace_version(RootVersion),
  version_with_offset(RootVersion, Offset, ExpectedVersion).

gen_enforced_field(WorkspaceCwd, 'version', RootVersion) :-
  root_workspace_version(RootVersion),
  workspace_field(WorkspaceCwd, 'version', _),
  workspace_ident(WorkspaceCwd, Ident),
  \+ special_workspace_offset(Ident, _).
