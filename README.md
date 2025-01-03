# Patterns of Modern App Development

At their core, modern applications are built on a set of common building blocks. Applications may use one or all of the building blocks. They are:

- Web-facing applications (APIs, web frontends)
- background applications (queue processors, asynchronous processes, event handlers)
- storage (databases, blob storage)
- integration (queues, topics, buses, streams)
- caches (because ya'know performance)
- [observability](#observability) (because ya'know things break)
- TODO: service to service communication
- TODO: orchestration

This repository aims to explore these different building blocks and their patterns and combinations. It will do all of that using serverless technologies.

But first, let's define what serverless actually means in this context.

## Serverless? What does it even mean

Historically, serverless has been defined by a set of core principles. [Momento](https://gomomento.com) wrote a great article introducing the [Litmus Test for Serverless](https://www.gomomento.com/blog/fighting-off-fake-serverless-bandits-with-the-true-definition-of-serverless/). For a service to be considered serverless it must:

1. Have nothing to provision or manage
2. Usage-based pricing with no minimums
3. Ready with a single API call
4. No planned downtime
5. No instances

I like this definition, but I would add one caveat. Viewing serverless as 'all or nothing'—either serverless or not—can remove some valuable services.

Take a service like Azure Container Apps (ACA). ACA is a container orchestrator that provides an abstraction on top of Kubernetes. You can deploy an application by providing a container image, CPU/memory requirements and scaling behaviour if required. **There is almost 0 operational overhead running an application this way**.

Looking at the Litmus test, this meets the criteria of 1, 3, 4 and 5. 2 gives us nuance. An application running on ACA won't automatically scale to zero; you can configure scaling rules, but it doesn't 'just happen'. When stopped, they don't cost you anything. You pay only when your app is running. But your app is running _all the time_ even if no requests are coming in.

This application is still serverless. No, it doesn't automatically scale to zero. Yes, you would pay for the application running when no requests are coming in. **But you can deploy an application with next to 0 operational overhead**.

> Serverless is a spectrum, not a binary decision. You can be more or less serverless based on the requirements of your application.

In this repository, I want to demonstrate how to run modern web applications on various cloud providers with little to no operational overhead.

### What does it mean for you?

If you're a developer, at least anything like me, you want to run your application with as little infrastructure worries as possible. _"Here's my application. It needs this CPU and memory, scale it like this, and frankly, I don't care about anything else."_

If your company has invested time, energy, and money into building a Kubernetes platform, then great. That can _feel_ serverless to you as a developer; leverage it. This isn't to say Kubernetes isn't valuable—it very much is. But it's valuable when your application needs it.

> If your ability to dynamically scale and manage infrastructure is a core differentiator for your business and your customers, great. Go for it. Otherwise, you don't need Kubernetes.

Are you training a machine learning model, doing heavy GPU computation, or ingesting dynamic/large amounts of data? Virtual machines and Kubernetes are probably useful.

The conversation around managed services vs. Kubernetes becomes more interesting when you zoom out and look at the bigger picture. Does your organisation have an excellent reason to invest in building a Kubernetes platform (which is just rebuilding Cloud Run/Fargate/Container Apps)? If you have a good reason to do it (that isn't CV-driven development), then great.

Otherwise, use a managed serverless, be as serverless as possible, and build your application in a way that keeps you portable.

## The Application

The application in question is written in Rust and used to manage loyalty points for a fictional eCommerce company. A background process receives events from an `order-completed` Kafka topic, processes the event, and stores loyalty point information in a Postgres database.

A separate web application exposes two endpoints: one to GET a customer's current loyalty account information and a second to spend (POST) loyalty points. These two applications run as separate containers connecting to the same database.

![Architecture Diagram](assets/arch-diagram.png)

The internals of the application are minor; just know that one exposes a web app, one runs a background process handling events, and there is shared storage (Postgres).

Let's now take a look at how you can run this application:

### Code Structure

One of the common pushbacks to adopting serverless technologies, or managed services in general, is the dreaded vendor lock-in. Whilst this is certainly something to consider, all your decisions give you a form of lock-in. Building with Rust? Language lock-in. Using MongoDB? Database lock-in. Lock-in is unavoidable. You have control over this as a developer though?

The ports & adapters or hexagonal architecture programming styles seem like a lot of ceremony. You need to write a lot of additional code, which adds misdirection and complexity to your code base. But it is precisely these patterns that help you avoid lock-in. Take the code structure for this application:

```
- adapters
 - adapters.rs
 - lib.rs
- backend
 - main.rs 
- core
 - lib.rs
 - loyalty.rs
- web
 - main.rs
```

The backend and web directories are the entry points for the respective applications. These directories set up the host and configure any host-specific stuff (HTTP routes, Kafka connections). Both application hosts then use the `adapters` library to configure data access and the `core` library to call into the business logic. The core library contains the main business logic [`loyalty.rs`](./src/core/src/loyalty.rs) and `traits` that are implemented in the adapters crate.

For example the [`LoyaltyPoints`](./src/core/src/loyalty.rs#194) is implemented in [`PostgresLoyaltyPoints` in](./src/adapters/src/adapters.rs#24). This means 

```Rust
pub(crate) trait LoyaltyPoints {
    async fn new_account(
 &self,
        customer_id: String,
 ) -> anyhow::Result<LoyaltyAccount, LoyaltyErrors>;
    async fn retrieve(&self, customer_id: &str) -> anyhow::Result<LoyaltyAccount, LoyaltyErrors>;
    async fn add_transaction(
 &self,
        account: &LoyaltyAccount,
        transaction: LoyaltyAccountTransaction,
 ) -> anyhow::Result<(), LoyaltyErrors>;
}
```

In its simplest form, the `loyalty.rs` file contains business logic and traits (or interfaces if you're unfamiliar with Rust). The `adapters` library contains the actual implementation:

What benefit does this give you? As you can see in the repository, if you need to port part of the application to a different compute provider... In this case, say it's Lambda, it's as simple as adding a new [entrypoint](./src/backend-lambda/). That single package contains the infrastructure specifics, and then you can simply call the core library.

Similarly, if you needed to switch your database provider (I hope you don't need to), you would need to change `adapters.rs`, and the rest of your code/business logic would stay the same. _I realise this doesn't account for the pain of a data migration_.

In this repository you'll find an example of this same application running in [Cloudflare Workers](#cloudflare-workers). In Cloudflare, it uses a different databases, messaging system and compiles to web assembly. It runs with zero changes to the core application code. It's simply a seperate implementation of the `LoyaltyPoints` trait.

A well-structured code base that separates applications from infrastructure will help you avoid lock-in. Also, aim for stateless compute.

Right, on to deployment, then...

## Prerequisites

> [!CAUTION]
> Deploying resources in this repository may incur costs in your cloud accounts account. Each provider specific section contains instructions on deleting all resources, it is recommended you do this when not in use.

When you deploy the application to one of the various cloud providers detailed below, you must have a Postgres database and a Kafka cluster with a topic called `order-completed`. Of course, you can set up a Kafka cluster and Postgres-compatible database however you choose. However, I'd highly recommend checking out:

- Docker (for local dev)
- [Neon for Postgres](https://neon.tech/)
- [Confluent Cloud for Kafka](https://www.confluent.io/)
- [Momento for caching](https://www.gomomento.com/)
- [Make](https://formulae.brew.sh/formula/make)

Confluent, Neon and Momento all have free tiers that you can use to provision **serverless** (yes, I said it) Kafka clusters, Postgres databases and caches. Neon is the closest I've seen to a fully serverless database service.

## Local

You can start up the entire application locally, both for local dev and for running integration tests:

```sh
# Configure the required environment variable to send OTEL data to Jaeger running locally
docker compose -f docker-compose-all.yml up -d
cargo run --package loyalty-web
cargo run --package loyalty-backend
```

Once running locally, you can access the API endpoint on `http://localhost:8080`. The `docker-compose-all` Docker compose file also starts up a 'simulator' that simulates load against the endpoints. If you open your web browser to the [Jaeger UI](http://localhost:16686/) you will see telemetry information.

## AWS

The various different deployment options use different IaC tools. However, whichever you choose, you will always need to set some environment variables on your machine:

```sh
# The Kafka broker to use when deployed
export BROKER=
# The Kafka username
export KAFKA_USERNAME=
# The Kafka password
export KAFKA_PASSWORD=
# The database URL for the Postgres database in the format 'postgresql://postgres:mysupersecretlocalpassword@localhost/loyalty'
export DATABASE_URL=
# Optional: Enable Datadog for instrumentation
export DD_API_KEY
```

Once the environment variables are set, you can use any deployment methods below.

### Deploy ECS Fargate

```sh
make deploy-ecs
```

### Deploy Lambda

```sh
make deploy-lambda
```

## Azure

The Azure Container Apps deployment uses Terraform as the IaC tool. You must create a [dev.tfvars](./azure-container-apps/dev.tfvars) file under `./azure-container-apps/dev.tfvars`.

You can also deploy Azure Container Apps using the `az containerapp up` command in the Azure CLI.

Create dev.tfvars file

```tf
env             = ""
dd_site         = ""
dd_api_key      = ""
subscription_id = ""
database_url    = ""
kafka_broker    = ""
kafka_username  = ""
kafka_password  = ""
app_version = "f835e5d"
```

Then deploy

```sh
az login
make deploy-aca
```

## GCP

The Google Cloud Run deployment uses Terraform as the IaC tool. You must create a [dev.tfvars](./google-cloud-run/dev.tfvars) file under `./azure-container-apps/dev.tfvars`.

### Cloud Run

Create dev.tfvars file

```tf
live_image_tag = "f835e5d"
canary_image_tag = ""
canary_enabled = false
canary_percent = 10
force_new_revision = false
env             = ""
dd_site         = ""
dd_api_key      = ""
kafka_broker    = ""
kafka_username  = ""
kafka_password  = ""
database_url    = ""

```

Then deploy

```sh
gcloud auth login
gcloud auth application-default login
make deploy-cloud-run
```

## Cloudflare Workers

Cloudflare workers are a relatively new technology in the world of modern app development, but they are a powerful one. Using the Javascript V8 runtime under the hood they are incredibly fast and lightweight to run. It also means that if your code compiles to web assembly, it can run inside Cloudflare Workers. 

For us Rust developers, this is pretty useful. There is a crate maintained by Cloudflare ([workers-rs](https://github.com/cloudflare/workers-rs)) that makes gives you the required types and macros to write a Rust application that will run on Cloudflare Workers.

The compilation to web assembly does add it's challenges. But remember, you've structured your application code in a way that makes the choice of compute largely irrelevant. Running on Cloudflare is as simple as adding a new ['entry point'](./src/cloudflare/) for your application and then hacking around with any non WASM supported parts of Rust. 

On that note, the Cloudflare implementation of the loyalty point application does use native Cloudflare services. Namely, the [D1 database](https://developers.cloudflare.com/d1/) service and [Cloudflare Queues](https://developers.cloudflare.com/queues/). This is primarily to simplify the compilation to web assembly, but it also demonstrates how even shifting your database provider is relatively straighforward (ok, forget about the data migration part) is as simple as just adding a new [adapter](./src/cloudflare/src/adapters.rs).

There are limitations to running Rust inside Cloudflare, and those limitations are pretty well documented inside the [Github Repo](https://github.com/cloudflare/workers-rs?tab=readme-ov-file#faq).

### Deploy to Cloudflare

You will need to ensure you have an account with Cloudflare, and have installed the [Wrangler CLI](https://developers.cloudflare.com/workers/wrangler/install-and-update/). 

You'll also need to deploy the D1 database and queues. You can do that by running the below `make` command

```sh
make cloudflare-prereqs
```

After deployment, you will see a `database_id` property in your terminal. Copy that ID, and paste it into the [`wrangler.toml`](./src/cloudflare/wrangler.toml):

```toml
[[d1_databases]]
binding = "DB"
database_name = "patterns-of-modern-apps"
database_id = "<database_id_goes_here>"
```

Once you have copied over the database ID, run:

```sh
make deploy-cloudflare
```

And there you have it, you'll receive a URL back in the terminal after deployment. You can call that in the same way you would on any other hosting platform. You can also manually send messages onto the queue using the Cloudflare console.

To teardown all created resources, run:

```sh
make destroy-cloudflare
```

## Fly.IO

[Fly.IO](https://fly.io) uses a custom CLI tool for deployment. First, ensure you have [installed the Fly CLI](https://fly.io/docs/flyctl/install/). Then, use the below commands to create the application and its secrets and then run the deployment.

```sh
fly app create --name loyalty-web
fly app create --name loyalty-backend
```

### Secrets

```sh
fly secrets set -a loyalty-web DATABASE_URL=""
fly secrets set -a loyalty-backend DATABASE_URL=""
fly secrets set -a loyalty-backend BROKER=""
fly secrets set -a loyalty-backend KAFKA_USERNAME=""
fly secrets set -a loyalty-backend KAFKA_PASSWORD=""
```

### Deploy

```sh
fly deploy -c fly-web.toml
fly deploy -c fly-backend.toml
```

## Observability

All the sample applications defined here deploy a [Datadog Agent](https://docs.datadoghq.com/agent/?tab=Linux) as a sidecar, or as a Lambda extension. The application itself, is configured to OpenTelemetry compatible data to an endpoint defined by the `OTLP_ENDPOINT` environment variable. If you want to use OpenTelemetry, you can either set the `OTLP_ENDPOINT` environment variable to be a different endpoint, or replace the Datadog sidecar with a full OpenTelemetry collector (or in the case of Lambda, the ADOT collector).

For example, in the Azure Container Apps example you would replace this code with your OTEL collector.

```t
container {
       name   = "datadog"
       image  = "index.docker.io/datadog/serverless-init:latest"
       cpu    = 0.25
       memory = "0.5Gi"

       env {
              name  = "DD_SITE"
              value = var.dd_site
       }
       env {
              name  = "DD_ENV"
              value = var.env
       }
       env {
              name        = "DD_API_KEY"
              secret_name = "dd-api-key"
       }
       env {
              name  = "DD_VERSION"
              value = var.app_version
       }
       env {
              name  = "DD_AZURE_SUBSCRIPTION_ID"
              value = data.azurerm_subscription.primary.subscription_id
       }
       env {
              name  = "DD_AZURE_RESOURCE_GROUP"
              value = azurerm_resource_group.modern_apps_container_apps.name
       }
       env {
              name  = "DD_OTLP_CONFIG_RECEIVER_PROTOCOLS_GRPC_ENDPOINT"
              value = "0.0.0.0:4317"
       }
       env {
              name  = "DD_APM_IGNORE_RESOURCES"
              value = "/opentelemetry.proto.collector.trace.v1.TraceService/Export$"
       }
}
```
