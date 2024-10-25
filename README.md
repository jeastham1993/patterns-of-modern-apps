# Patterns of Modern App Development

At their core, modern applications are built of a set of common building blocks. Applications may use one, or all of the building blocks. They are:

- web facing applications (APIs, web frontends)
- background applications (queue processors, asynchronous processes, event handlers)
- storage (databases, blob storage)
- integration (queues, topics, buses, streams)
- caches (because, performance!)

This repository aims to explore these different building blocks, and the patterns and combinations. And it is going to do all of that using serverless technologies, for all of the different building blocks.

But first, let's define what serverless actually means in this context.

## Serverless? What does it even mean

Historically, serverless has been defined by a set of core principles. [Momento](https://gomomento.com) wrote a great article introducing the [Litmus Test for Serverless](https://www.gomomento.com/blog/fighting-off-fake-serverless-bandits-with-the-true-definition-of-serverless/). For a service to be considered serverless it must:

1. Have nothing to provision or manage
2. Usage-based pricing with no minimums
3. Ready with a single API call
4. No planned downtime
5. No instances

I like this definition, but there is one caveat I would add. Viewing serverless as 'all or nothing', either your serverless or your not, can remove some useful services.

Take a service like Azure Container Apps (ACA). ACA is a container orchestrator that provides an abstraction on top of Kubernetes. You can deploy an application by just providing a container image, CPU/memory requirements and scaling behaviour if required. **There is almost 0 operational overhead running an application in this way**.

Looking at the Litmus test, this meets the criteria of 1, 3, 4 and 5. 2 gives us nuance. An application running on ACA won't automatically scale to zero, you can configure scaling rules but it doesn't 'just happen'. When stopped, they don't cost you anything. You pay only when your app is running. But your app is running *all of the time* even if there aren't any requests coming in. 

This application is still serverless. No, it doesn't automatically scale to zero. Yes, you would be paying for the application running when no requests are coming in. **But you can deploy an application with next to 0 operational overhead**.

> Serverless is a spectrum, not a binary decision. You can choose to be more or less serverless based on the requirements of your application

This is what I want to demonstrate in this repository. How you can run modern web applications on various different cloud providers, and do so in a way with little to no operational overhead.

### What does it mean for you?

For you as an individual developer, at least if you're anything like me, you want to run your application with as little infrastructure worries as possible. "Here's my application, run it with this CPU and memory, scale it like this and frankly I don't care about anything else". If your company has invested time, energy and money into building a Kubernetes platform then great.

The conversation around managed services vs Kubernets becomes more interesting when you zoom out and look bigger picture. Does your organisation as a whole have a good reason to invest in building a Kubernetes platform (which honestly is just rebuilding Cloud Run/Fargate/Container Apps). If you have a good reason to do it (that isn't CV driven development) then great do it. 

But otherwise, use a managed serverless, be as serverless as possible, and build your application in a way that keeps you portable.

## The Application

The application in question is written in Rust, and is used for managing loyalty points for a fictional eCommerce company. A background process receives events from an `order-completed` Kafka topic, processes the event and stores loyalty point information in a Postgres database.

A seperate web application exposes two endpoints, one to GET current loyalty account information for a customer and a second to spend (POST) loyalty points. These two applications run as seperate containers, both connecting to the same database.

![Architecture Diagram](assets/arch-diagram.png)

The internals of the application aren't important, just know one exposes a web app, one runs a background process handling events, and there is shared storage (Postgres).

Let's now take a look at how you can run this application:

## Prerequisites

When you deploy the application to one of the various cloud providers detailed below, you will need to have a Postgres database and a Kafka cluster with a topic called `order-completed`. Of course, you can setup a Kafka cluster and Postgres compatible database in whatever way you choose. However, I'd highly recommend checking out:

- [Neon for Postgres](https://neon.tech/)
- [Confluent Cloud for Kafka](https://www.confluent.io/)
- [Momento for caching](https://www.gomomento.com/)

Confluent, Neon and Momento all have free tiers that you can use to provision **serverless** (yes, I said it) Kafka clusters, Postgres databases and caches. Neon is actually the closest I've seen to a fully serverless database service.

## Local

TODO: Add local dev instructions

## AWS

TODO: Docs on setup

### Deploy ECS Fargate

```sh
cd ecs-fargate
cdk deploy
```

### Deploy Lambda

```sh
sam build
sam deploy --guided
```

## Azure

TODO: Docs on setup

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
```

Then deploy

```sh
cd azure-container-apps
az login
terraform init
terraform apply --var-file dev.tfvars
```

## GCP

TODO: Docs on setup

Uses minimum instances and CPU always on for background processing appplication to allow Kafka consumption.

### Cloud Run

```sh

```

## Fly.IO

TODO: Docs on setup

```sh
fly app create --name loyalty-web
fly app create --name loyalty-backend
fly deploy
```

### Secrets

```sh
fly secrets set -a loyalty-web DATABASE_URL=""
fly secrets set -a loyalty-backend DATABASE_URL=""
fly secrets set -a loyalty-backend BROKER=""
fly secrets set -a loyalty-backend KAFKA_USERNAME=""
fly secrets set -a loyalty-backend KAFKA_PASSWORD=""
```
