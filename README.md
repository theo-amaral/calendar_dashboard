# calendar_dashboard

Dashboard leve do **Google Calendar** e do **Google Tasks** para o terminal, escrito em Rust.

Ao rodar, ele mostra uma caixa com uma saudação, os eventos das próximas 24 horas e as tarefas dos próximos 3 dias. Sem abrir o navegador e sem sair do terminal.

```
╭─── Bom dia, Theo. ───────────────────────────────────────────────────────╮
│    Eventos de hoje:  Aula de Controle 10h00 às 11h40   Reunião Dia Todo  │
│    Tarefas dos próximos 3 dias:  Entregar EP2   Estudar para a prova     │
╰──────────────────────────────────────────────────────────────────────────╯
```

*(Exemplo sem as cores. Na tela real, cada evento e cada tarefa aparece como um bloco colorido.)*

## O que ele mostra

- **Saudação** que muda conforme a hora do dia (bom dia, boa tarde, boa noite).
- **Eventos de hoje**: tudo o que começa nas próximas 24 horas, juntando **todos os seus calendários**, em ordem de horário. Eventos com hora mostram o intervalo (`10h00 às 11h40`), e os de dia inteiro mostram `Dia Todo`. Cada calendário usa uma cor.
- **Tarefas dos próximos 3 dias**: as tarefas ainda não concluídas da sua lista padrão do Google Tasks, com vencimento em até 3 dias.
- Quando não cabe tudo na largura do terminal, aparece um contador `+N` com quantos itens ficaram de fora. Sem eventos ou sem tarefas, ele avisa com uma mensagem.

## Requisitos

- [Rust](https://rustup.rs/) **1.82 ou mais recente**
- Uma conta Google
- Credenciais OAuth próprias (veja abaixo)
- Um terminal com **true color** e uma [Nerd Font](https://www.nerdfonts.com/), para os blocos coloridos aparecerem com as pontas arredondadas

## Configuração

O programa usa as APIs oficiais do Google, então precisa de credenciais criadas por você:

1. Entre no [Google Cloud Console](https://console.cloud.google.com/) e crie um projeto.
2. Ative a **Google Calendar API** e a **Google Tasks API** nesse projeto.
3. Configure a tela de consentimento OAuth e adicione o seu e-mail como **usuário de teste**.
4. Em *Credenciais*, crie um **ID do cliente OAuth** do tipo **Aplicativo para computador** e baixe o JSON.
5. Salve o arquivo como `client_secret.json` em `~/.config/calendar_dashboard/` (crie a pasta se ela não existir).

Na primeira execução, o programa abre o navegador para você autorizar o acesso. Ele recebe a resposta do Google em `http://127.0.0.1:8888`, então essa porta precisa estar livre. Os tokens ficam salvos em `~/.config/calendar_dashboard/tokens.json` e são renovados automaticamente.

> Enquanto o app estiver em modo de teste no Google Cloud, o Google expira a autorização depois de alguns dias. Se o login parar de funcionar, apague o `tokens.json` e rode de novo para autorizar outra vez.

## Como usar

```bash
git clone https://github.com/theo-amaral/calendar_dashboard
cd calendar_dashboard
cargo run --release
```

Para instalar e chamar de qualquer lugar:

```bash
cargo install --path .
calendar_dashboard
```

Uma ideia é colocar `calendar_dashboard` no fim do seu `.bashrc` ou `.zshrc`, para ver a agenda toda vez que abrir o terminal.

Para ver as mensagens de depuração: `RUST_LOG=debug calendar_dashboard`.

## Personalização

Algumas coisas estão fixas no código:

- **Nome na saudação e cores dos calendários**: em `src/main.rs` (`greeting` e `print_colors`). Mude para customizar o nome.
- **Janelas de tempo**: 24 horas para eventos (`src/main.rs`) e 3 dias para tarefas (`fetch_tasks`, em `src/lib.rs`).

## Segurança

`client_secret.json` e `tokens.json` dão acesso à sua agenda e às suas tarefas. São arquivos pessoais: **nunca os envie para o GitHub**. Por padrão eles ficam em `~/.config`, fora do repositório.

Se algum deles vazar, revogue o acesso em [myaccount.google.com/permissions](https://myaccount.google.com/permissions) e gere novas credenciais.

## Feito com

[tokio](https://tokio.rs/), [google_calendar](https://crates.io/crates/google_calendar), [reqwest](https://crates.io/crates/reqwest), [chrono](https://crates.io/crates/chrono), [colored](https://crates.io/crates/colored) e [termion](https://crates.io/crates/termion).

## Autor

Feito por [Theo Amaral](https://github.com/theo-amaral).
