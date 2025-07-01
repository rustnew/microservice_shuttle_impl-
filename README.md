
# Analytics Service

Ce micro-service Rust permet de recevoir des événements analytiques et de fournir des statistiques aggrégées, avec authentification par token (`Bearer`). Il est construit avec :

- **Actix Web** pour l'API HTTP
- **SQLx** pour l'accès PostgreSQL
- **Shuttle** pour le déploiement serverless
- Auth via header `Authorization: Bearer <API_KEY>`

---

## 📦 Installation / Développement local

1. Installe les dépendances :
   ```bash
   cargo install cargo-shuttle sqlx-cli
````

2. Configure la base de données :

   ```bash
   export DATABASE_URL="postgres://postgres:password@localhost:5432/analytics_db"
   sqlx database create
   sqlx migrate run
   ```

3. Crée un secret dans Shuttle (ou local `.env`) pour `ANALYTICS_API_KEY`.

4. Lance localement avec :

   ```bash
   shuttle run
   ```

---

## 🔧 Endpoints

| Méthode | Route     | Description                          | Auth requise |
| ------: | :-------- | :----------------------------------- | :----------- |
|     GET | `/health` | Vérifie que le service est en ligne  | ❌            |
|    POST | `/events` | Reçoit un `AnalyticsEvent` JSON      | ✔️           |
|     GET | `/stats`  | Retourne un tableau des `EventStats` | ❌            |

### Payload `AnalyticsEvent`

```json
{
  "event_type": "string",
  "post_id": 123,               // optionnel
  "data": { "key": "value" }    // JSON libre
}
```

### `EventStats` retourné

```json
[
  { "event_type": "click", "count": 42 },
  { "event_type": "view",  "count": 10 }
]
```

---

## 🧠 Authentification

Un middleware applique la logique suivante :

* Lit le header : `Authorization: Bearer <token>`
* Compare `<token>` à la clé `ANALYTICS_API_KEY`
* Si valide → continue la requête
* Sinon → réponse `401 Unauthorized`

La logique est implémentée avec `wrap_fn(...)` d’après \[la doc Actix middleware]\([docs.shuttle.dev][1], [stackoverflow.com][2], [shuttle.dev][3], [users.rust-lang.org][4], [actix.rs][5]).

---

## 🧩 Structure du code

* `AppState` contient `db: PgPool` et `api_key: String`
* Handlers :

  * `receive_event` → insère un événement dans la table `events`
  * `get_stats` → agrège `event_type` + nombre
  * `health` → simple check `200 OK`
* Middleware `auth_middleware` appliqué uniquement à `/events`

---

## 🚀 Déploiement via Shuttle

Le point d'entrée est géré par :

```rust
#[shuttle_runtime::main]
async fn main(
    #[shuttle_shared_db::Postgres] pool: PgPool,
    #[shuttle_runtime::Secrets] secrets: SecretStore,
) -> ShuttleActixWeb<impl FnOnce(&mut ServiceConfig) + Send + Clone + 'static> { ... }
```

Shuttle crée et provisionne automatiquement une DB, injecte `PgPool` et la clé `ANALYTICS_API_KEY` depuis la console ([shuttle.dev][3], [docs.shuttle.dev][1], [docs.rs][6]).

---

## 🧪 Exemple `Cargo.toml`

```toml
[dependencies]
actix-web = "4"
sqlx = { version = "0.8.6", features = ["runtime-tokio", "tls-native-tls", "postgres", "json"] }
serde = { version = "1.0", features = ["derive"] }
shuttle-runtime = "0.55"
shuttle-actix-web = "0.55"
shuttle-shared-db = { version = "0.55", features = ["postgres","sqlx"] }
```

---

## ✔️ Astuces

* Middleware avec `wrap_fn`: simple et idiomatique ([users.rust-lang.org][7])
* `sqlx::query_as!` compile la requête au build time avec sécurité de type
* `unwrap_or_default()` sur les stats → erreurs SQL ne font pas planter l’API

---

## 🍀 Contribution

Les contributions sont les bienvenues. Ouvre une issue ou une PR si tu veux :

* ajouter un logging structuré (`tracing`),
* améliorer la gestion des erreurs (en renvoyant un 500 si fetch\_stats échoue),
* ou ajouter des filtres par `post_id`.

---

## 💡 Ressources utiles

* Middleware wrap\_fn → docs Actix Web ([lib.rs][8], [cseweb.ucsd.edu][9], [actix.rs][5])
* Deploy Actix + Shuttle → exemple officiel ([docs.rs][6])
* Documentation Actix sur les DB pools ([actix.rs][10])
