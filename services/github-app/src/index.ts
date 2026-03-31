import { Probot } from "probot";

export default (app: Probot) => {
  app.on("issues.opened", async (context) => {
    const issueComment = context.issue({
      body: "Thanks for opening this issue!",
    });
    await context.octokit.issues.createComment(issueComment);
  });

  app.on(["pull_request.opened", "pull_request.synchronize"], async (context) => {
    const pr = context.payload.pull_request;
    const owner = context.payload.repository.owner.login;
    const repo = context.payload.repository.name;
    const sha = pr.head.sha;
    const prNumber = pr.number;

    // 1) Create a check run (queued)
    const check = await context.octokit.checks.create({
      owner,
      repo,
      name: "Preview deploy",
      head_sha: sha,
      status: "in_progress",
      output: {
        title: "Preview deploy started",
        summary: `Dispatching preview deploy for PR #${prNumber}`,
      },
    });

    // 2) Dispatch workflow in the app repo
    await context.octokit.actions.createWorkflowDispatch({
      owner,
      repo,
      workflow_id: "preview-deploy.yml",
      ref: pr.head.ref,
      inputs: {
        pr_number: String(prNumber),
        sha,
        check_run_id: String(check.data.id),
      },
    });
  });

  app.on(["pull_request.closed"], async (context) => {
    const pr = context.payload.pull_request;
    const owner = context.payload.repository.owner.login;
    const repo = context.payload.repository.name;

    await context.octokit.actions.createWorkflowDispatch({
      owner,
      repo,
      workflow_id: "preview-destroy.yml",
      ref: pr.base.ref,
      inputs: {
        pr_number: String(pr.number),
      },
    });
  });

  // For more information on building apps:
  // https://probot.github.io/docs/

  // To get your app running against GitHub, see:
  // https://probot.github.io/docs/development/
};
