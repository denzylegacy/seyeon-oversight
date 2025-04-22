# Executando Seyeon Oversight com Docker

Este documento contém instruções para executar o sistema Seyeon Oversight usando Docker e Docker Compose.

## Pré-requisitos

- [Docker](https://docs.docker.com/get-docker/) instalado
- [Docker Compose](https://docs.docker.com/compose/install/) instalado

## Configuração

1. Crie um arquivo `.env` baseado no exemplo fornecido:

```bash
cp .env.example .env
```

2. Edite o arquivo `.env` e adicione suas credenciais e configurações:

```bash
nano .env  # ou use seu editor de texto preferido
```

### Variáveis de ambiente necessárias:

- `REDIS_URL`: URL de conexão com o Redis (já configurado para o container)
- `CRYPTOCOMPARE_API_KEY`: Sua chave de API do CryptoCompare
- `RAPIDAPI_KEY`: Sua chave de API do RapidAPI para o Fear & Greed Index
- `SMTP_FROM_EMAIL`: Email de origem para envio de alertas
- `SMTP_TO_EMAIL`: Email de destino para alertas
- `SMTP_CC_EMAILS`: Lista de emails CC separados por vírgula
- `SMTP_PASSWORD`: Senha SMTP ou senha de aplicativo (recomendado para Gmail)

## Construção e Execução

### Construir e iniciar os containers

```bash
docker-compose up -d --build
```

A flag `-d` executa os containers em modo detached (segundo plano).

### Verificar logs da aplicação

```bash
docker logs -f seyeon-oversight
```

### Parar os containers

```bash
docker-compose down
```

### Reiniciar os containers

```bash
docker-compose restart
```

## Persistência de Dados

Os dados do Redis são persistidos no volume Docker `redis-data` e sobreviverão a reinicializações do container.
