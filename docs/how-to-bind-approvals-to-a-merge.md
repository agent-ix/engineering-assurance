---
title: Bind approvals to a merged change
type: how-to
description: Record approvals against the reviewed head and the merge result.
tool_version: 0.11.1
---

<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (C) 2026 Agent-IX -->

# Bind approvals to a merged change

Record approvals against the head commit and merge result reported by the pull request source.

## Before you start

Load the role policy, submitted reviews, account types, and approver names from the approval source. Prepare a statement signer, record sink, and record IDs.

## Steps

1. Read the pull request lifecycle state from the source. Use `PullRequestState::Open` while it is open and `PullRequestState::Closed` when it closed without merging.
2. For a merged request, read the head at merge and the merge-result commit. Use the merge commit, squash commit, or rebased tip reported by the source.
3. Set `request.pull_request` to the state reported by the source and pass the request to the writer:

   ```rust,no_run
   use engineering_assurance::approval_statement_writer::{
       ApprovalStatementSigner, ApprovalStatementWriteRequest, PullRequestState,
       StatementWriteError, WrittenApprovalStatement, write_approval_statements,
   };
   use engineering_assurance::record_sink::sink::RecordSink;

   fn write_for_source_state<'a>(
       sink: &dyn RecordSink,
       signer: &dyn ApprovalStatementSigner,
       mut request: ApprovalStatementWriteRequest<'a>,
       state: PullRequestState<'a>,
   ) -> Result<Vec<WrittenApprovalStatement>, StatementWriteError> {
       request.pull_request = state;
       let (_, statements) = write_approval_statements(sink, signer, request)?;
       Ok(statements)
   }
   ```

   For a merged request, construct `PullRequestState::Merged` with the repository, pull request reference, head at merge, and merge-result commit.

## Expected output

Each statement names the merge result as its subject and preserves the reviewed head in its predicate:

```json
{
  "subject": [{
    "name": "example/widget",
    "digest": {"gitCommit": "2222222222222222222222222222222222222222"}
  }],
  "predicate": {
    "head_commit": "1111111111111111111111111111111111111111",
    "merged_commit": "2222222222222222222222222222222222222222"
  }
}
```

The statement also carries the approval identity, role, meaning, source timestamp, and pull request reference.

An open or unmerged closed request returns `NotMerged` before signing or publishing. A review on a different head produces no statement, even when its tree matches the merge result. A rebase uses the rebased tip as the merge result.
