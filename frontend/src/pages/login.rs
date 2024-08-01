use leptos::*;
use leptos_router::use_navigate;

use crate::{
    components::*,
    errors::*,
    grpc,
    session::{use_token_info, TokenInfo},
};

#[component]
pub fn Login() -> impl IntoView {
    let username = create_rw_signal("".to_owned());
    let password = create_rw_signal("".to_owned());

    let submit =
        create_action(move |(username, password): &(String, String)| {
            let username = username.clone();
            let password = password.clone();

            let navigate = use_navigate();
            let (_, set_token_info) = use_token_info();
            async move {
                let mut token_set =
                    grpc::token_client::TokenClient::new(grpc::new_client());
                let resp = token_set
                    .create(grpc::LoginRequest {
                        username,
                        password,
                        expiry: None,
                        request_id: None,
                    })
                    .await?;
                let resp = resp.into_inner();
                set_token_info(Some(TokenInfo {
                    token: resp.token,
                    role: grpc::Role::try_from(resp.role).map_err(|err| {
                        Error {
                            kind: ErrorKind::Internal,
                            context: "API error, please check API version."
                                .to_owned(),
                        }
                        .context(err.to_string().as_str())
                    })?,
                }));
                navigate("/", Default::default());
                Result::<_>::Ok(())
            }
        });

    let disabled = Signal::derive(move || {
        submit.pending()() || username().is_empty() || password().is_empty()
    });

    let error_msg = move || {
        let Some(Err(err)) = submit.value()() else {
            return "".to_owned();
        };
        match err.kind {
            ErrorKind::NotFound => {
                "Username or password is incorrect.".to_owned()
            }
            ErrorKind::RateLimit => {
                "You try too many times, please wait and try again".to_owned()
            }
            _ => {
                toast(view! { <p>{err.to_string()}</p> });
                "".to_owned()
            }
        }
    };

    view! {
        <main class="grow flex items-center justify-center">
            <form
                class="flex flex-col flex-nowrap justify-center min-w-80 w-1/4 p-4 bg-slate-900"
                on:submit=move |e| {
                    e.prevent_default();
                    submit.dispatch((username(), password()));
                    password.set("".to_owned());
                }
            >

                <div class="flex justify-center">
                    <img src="https://placehold.co/200" alt="Logo" class="max-w-64"/>
                </div>

                <div class="pt-4 flex flex-col">
                    <label for="username" class="text-text pb-2">
                        Username
                    </label>
                    <Input attr:id="username" value=username/>
                </div>
                <div class="pt-4 flex flex-col">
                    <label for="password" class="text-text pb-2">
                        Password
                    </label>
                    <Input variant=InputVariant::Password attr:id="password" value=password/>
                </div>
                <p class="w-full text-red text-center">{error_msg}</p>
                <div class="pt-4 w-full">
                    <Button type_="submit" class="w-full" disabled>
                        Login
                    </Button>
                </div>
            </form>
        </main>
    }
}
