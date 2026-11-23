## Tribe rules

These rules are for all nodes.

### Folder owners

Each folder has one owner squad. Replace this table with the folders of your
project.

| Folder | Owner |
|---|---|
| `src/ui/`, theme tokens, and the texts of the UI | ui |
| `tests/screenshots/` | ui |
| `src/domain/`, `src/data/`, and the other app code | core |
| the build files and the dependency manifest | core |
| `tests/` outside `tests/screenshots/` and `tests/lab/` | core |
| `experiments/` and `tests/lab/` | lab |
| the plan and the spec of an issue | the squad of the issue; for an issue of more than one squad, the squad that the director names |
| `.agent-graph/` and the agent instructions of the project | the director gives each change to a lead |

To change a file that another squad owns, ask the director. The director
sends the request to the owner squad.

In a squad, each file has one owner in each step. The lead, the dev, and the
checker of a squad share one worktree and one git index. Only the lead
commits, with the names of the files: `git commit -- <file>...`. For a new
file, run `git add <file>...` first. Nobody runs `git add -A`, `git add .`,
or `git commit -a`.

### Order

The message from the director to a lead gives:

- the issue, and the path of the worktree of the squad;
- for an issue of more than one squad, the contract between the squads: what
  the lab gives to core (measure, format, criterion), and what core gives to
  ui (state, events, interfaces);
- or a decision of the user, with the link to the approval;
- or the news that a pull request of another squad is merged.

### Report

The message from a lead to the director gives the pull request, what is done
or planned, the review findings that the lead accepted and refused, and what
the lead needs from the user. The lead uses technical words. The director
translates them for the user.

### Task

The message from a lead to a member gives:

- the path of the worktree, the branch, and the step of the plan;
- the files that the member can change;
- the conventions of the project that the member needs.

If a step needs the result of another step, the lead sends it after the other
step is complete.

### Result

The member sends back the changed files, a short summary, the result of the
tests that it ran, and its questions. A member does not commit, push, or talk
outside its squad.

### Approval of the user

The user approves a plan or a result in the session of the director, with the
words of the user. The director posts the approval on the pull request: what
is approved (the plan or the merge), the full SHA of the head of the branch,
and the exact words of the user. Only the director posts approvals.

Before it executes a plan or merges, the lead checks the approval. The
approval names the current head, or the branch moved only by clean merges of
the default branch. If the branch moved in another way, the lead asks the
director for a new approval. Nobody rebases or force-pushes a branch with an
approval.

### Merge queue

One merge at a time, in the order that the director gives. The lead reports
each merge. Then the director tells each other lead with an order, and lets
the next merge start. Each other lead merges the default branch into its
branch.

### Pull request of another squad

No branch starts from an open pull request of another squad. A squad can
write its spec at the same time. The code starts after the other pull request
is merged, from the default branch.

### Stuck session

A session that waits on a permission prompt cannot send a message. When the
director waits for a report, it pings the lead. If the lead does not answer in
10 minutes, the director tells the user which session to open and which
command to approve. The user can add the command to the permission list of
the project. A lead reports a member that does not answer to the director in
the same way.

### Urgent fix

The lead commits and pushes the current step, and tells its members to stop.
Then it switches the branch in the same worktree, makes the fix, and goes
back. The director decides which squad stops.

### Slice deadline

If a slice misses its deadline, or a host is down, the `scientist` reports it
to the director. Nobody runs the slice again on another host without a
decision of the director.

### Review dispute

A lead does a maximum of three review rounds. If blockers remain after the
third round, or if the lead does not agree with the review, the lead sends the
findings and its position to the director. The director gives the user the
options and a recommendation, and posts the decision of the user on the pull
request.
