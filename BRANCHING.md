# Branching and Releasing

`main` is the primary development branch in `full-service.git`.

Changes are typically created on topic branches, reviewed as PRs, and squash-merged into `main`.

Sometimes long running feature branches are created. These are, still, eventually
squash merged into `main`. (Alternatively, this could be a normal git merge
and not a squash merge, it doesn't much matter since the feature branch goes away
after it is merged.)

## Release branches

Major releases are started by creating a release branch e.g. `release/v4.0` off
of `main`.

Minor releases are started by creating a release branch e.g. `release/v4.1` off of
the predecessor `release/v4.0`.

Any bugs that are found are fixed in the release branch first.

Release candidates are tagged on release branches.

## Propagating changes

Changes which are made in release branches are propagated forward to main,
by _merging_ the release branch into main.

- The release branch is not deleted after this merge.
- This is a normal git merge and not a squash merge or a rebase merge.
  The purpose of this is to avoid git conflicts. ([More on this](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/incorporating-changes-from-a-pull-request/about-pull-request-merges#squashing-and-merging-a-long-running-branch))

Merging means that git will look at the state of the release branch, check if
any commits on it have not already been merged into main, and if not, attempt
to apply those diffs onto main (using 3-way merge conflict resolution).

After a commit from release branch has been merged this way, it will never be
considered by a future merge and can never create a conflict in a future merge.
(This is a major difference with squash merging and rebase merging. This is very
important when merging long-lived branches and not small PRs.)

If there are no conflicts, we can simply open a PR from e.g. `release/v4.0` to
`main` in github. The release branches are protected, so merging this PR will
not delete the branch.

If there are conflicts, this won't exactly work, because we will have to resolve
the conflicts, but we don't want to push more commits to `release/v4.0` to accomplish
this.

The local workflow for this may look something like this:

```
$ git status
On branch main
$ git checkout -b merge-release-v4.0
$ git merge release/v4.0

(resolve conflicts now)

$ git push origin merge-release-v4.0

(open pull request from this branch targetting main)
```

Here, we create a topic branch (off of main), merge the release branch into it,
resolve any conflicts, and then propose to merge this to main.

## When to merge the release branch

It is always desirable for the release branch to merge "cleanly" to main.
This ensures that all bug fixes (and changelog updates) in the release branch
were incorporated into main, and there can be no confusion about what fixes
made it back.

It is acceptable to merge the release branch to main at any of the following times:

- After making a change in the release branch
- On a periodic basis
- After finishing the release

Currently, we want this merge to happen via a PR and go through (usually perfunctory)
review. Reviewers should check that this is not a squash merge or a rebase merge.

Right now we think it's desirable that whenever the release branch moves forwards,
a PR could be opened automatically that proposes to merge the release branch to main,
exposing any git conflicts, which can then be reviewed and resolved in this PR.

## Making changes

Developers make changes using topic branches and PRs, and are free to
squash-merge their topic branches as they historically have. (It doesn't much
matter for this workflow whether or not topic branches are squash merged.)

When a developer wants to make a change, they should ask themselves, should this
go in the latest release branch, or in main?

- Changes which go to the release branch will eventually be merged forwards to
  main. No cherry-picking will be necessary. This is usually the right thing
  for a bug fix.
- Changes which should NOT go into the release branch should just go to main.

This decision tree covers the majority of changes, and does not involve any
cherry-picking.

Sometimes, a developer will make a PR to main, but then later realize it
should have gone to the release branch.

- At this point, you have to cherry-pick the change back to the release branch.
- You can cherry-pick your commit(s) from main.
- You should expect a git conflict to occur when release is merged into main
  after this. This is because a cherry-picked commit has a different hash from
  the original commit, and git does not consider them "the same" when merging,
  they are instead very similar commits touching the same code.
- You will have to resolve this conflict, which usually won't be too hard.
- To avoid this situation, make sure you consider whether a change should go
  to main or the release branch. If in doubt, ask in the channel.

This leads to a git history like the following:

[![](https://mermaid.ink/img/pako:eNqtU8FuwjAM_ZXIZ7aBaJbR86addto1F5OYNippq5CiIcS_L5QxlS2FViKnOO8lfn6O96AqTZACCysz_t1hncuStUtV1hofj5YOS5UzR2vCDT1tk0saMzqVgAlfSIghy2fx8g_xmAVomzxOO1BOqqgazyyaMi7FksuoV8mg6JzkejlqNu0pRwsxv6F5oMqbSn5Nmt0n4Yge81jpxPk8bspKiGRUI7uXvxYdq_-Uwvvd6kLO7R5qo4rIg4M-wxUzBym46Bcf-alPEUwgZA4sHUZ0f8Qk-JwsSUjDVqMrjq8eAg8bX33uSgWpdw1NoKk1eno1mDm0kK5wvQmnpI2v3Mdp5tvRPzPfWuSHePgGwSE-5w?type=png)](https://mermaid.live/edit#pako:eNqtU8FuwjAM_ZXIZ7aBaJbR86addto1F5OYNippq5CiIcS_L5QxlS2FViKnOO8lfn6O96AqTZACCysz_t1hncuStUtV1hofj5YOS5UzR2vCDT1tk0saMzqVgAlfSIghy2fx8g_xmAVomzxOO1BOqqgazyyaMi7FksuoV8mg6JzkejlqNu0pRwsxv6F5oMqbSn5Nmt0n4Yge81jpxPk8bspKiGRUI7uXvxYdq_-Uwvvd6kLO7R5qo4rIg4M-wxUzBym46Bcf-alPEUwgZA4sHUZ0f8Qk-JwsSUjDVqMrjq8eAg8bX33uSgWpdw1NoKk1eno1mDm0kK5wvQmnpI2v3Mdp5tvRPzPfWuSHePgGwSE-5w)

## Multiple concurrent releases

Should it become necessary to support multiple concurrent releases,
the workflow extends naturally:

- Changes target the _earliest_ release branch where they are relevant
- _Earlier_ release branches merge forward into the newer release branches
- The newest release branch merges into main.

Note that git merge handles well the semantics when a diamond pattern is created.
Suppose the following merges occur:

```mermaid
graph TD;
    A[release/v3.1]-->B[main];
    A-->C[release/v4.0];
    B-.->D[main];
    C-->D;
```

Even though a commit in `release/v3.1` may follow two paths into main branch,
it does not conflict with itself under a `git merge`. (This is another reason
why a normal git merge should always be preferred when merging two long-lived branches.)

## Why squash merge at all

The best argument for why topic branches should be squash merged is that,
often topic branches have a lot of small commits, and maybe only the last one
builds and passes tests. If we squash the PR into one commit, then we know that
the commit that lands on main builds and passes tests. If we do a merge commit,
then it's possible that during `git bisect` you will hit a commit that does not
build or pass tests.

In some projects, the convention is that all the commits in the PR should build
and pass tests, and you are supposed to use `git rebase` to either fix all the
commits or squash the ones that don't build.

However,

- Usually you won't run CI on every single commit, because it's prohibitive.
  So you will sometimes end up having commits under main that don't build.
- Simply squash merging it all is much less effort from the developer.
- More junior developers who aren't experts with `git rebase` won't have to learn
  all of those techniques.

Historically mobilecoin developers have preferred to click squash merge.

Squash merging also means that if you have to revert, your only option is to
revert the whole PR (and not an individual commit). However, historically that's
usually what we want to do.

Historically, we have rarely used `git bisect` to investigate regressions, but it
could be useful in the future.
