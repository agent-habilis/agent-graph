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

In a squad, each file has one owner in each step. All the sessions of a squad
share one git index. Only the lead commits, with the names of the files:
`git commit -- <file>...`. Nobody runs `git add -A`, `git add .`, or
`git commit -a`.

### Order

The message from the director to a lead gives:

- the issue;
- for an issue of more than one squad, the contract between the squads: what
  the lab gives to core (measure, format, criterion), and what core gives to
  ui (state, events, interfaces);
- or a decision of the user, with the link to the approval.

### Report

The message from a lead to the director gives the pull request, what is done
or planned, the review findings that the lead accepted and refused, and what
the lead needs from the user. The lead uses technical words. The director
translates them for the user.

### Task

The message from a lead to a member gives:

- the path of the repository, the branch, and the step of the plan;
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
words of the user. The director posts the approval on the pull request: the
command, the full SHA of the head of the branch, and the exact words of the
user. Before it executes a plan or merges, the lead checks that the approval
names the current head. If the branch moved, the lead asks the director for a
new approval.

### Merge queue

One merge at a time, in the order that the director gives. The director lets
the next merge start when the report of the last merge arrives. After a merge,
each other lead updates its branch from the default branch.

### Pull request of another squad

No branch starts from an open pull request of another squad. A squad can
write its spec at the same time. The code starts after the other pull request
is merged, from the default branch.

### Stuck session

A lead that waits on a permission prompt cannot send a message. The director
pings each lead that has an issue in progress. If a lead does not answer in 10
minutes, the director tells the user which session to open and which command
to approve. Then the command goes into the permission list of the project.

### Urgent fix

The lead finishes the current step, then switches the branch in the same
worktree, makes the fix, and goes back. The director decides which squad
stops.

### Review dispute

A lead does a maximum of three review rounds. If blockers remain after the
third round, or if the lead does not agree with the review, the lead sends the
findings and its position to the director. The director gives the user the
options and a recommendation, and posts the decision of the user on the pull
request.
