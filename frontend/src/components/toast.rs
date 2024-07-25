use leptos::*;
use leptos_icons::*;
use leptos_use::*;
use tailwind_fuse::tw_join;

turf::style_sheet!("src/components/toast.scss");

#[component]
pub fn ProvideToast(children: Children) -> impl IntoView {
    let toaster = create_rw_signal(Toaster::default());
    provide_context(toaster);
    let (toasts, _) = slice!(toaster.toasts);
    view! {
        {children()}
        <div class="fixed bottom-0 right-0 h-screen w-1/5 flex flex-col-reverse">
            <For
                each=toasts
                key=|toast| toast.0
                children=|(id, v)| view! { <Toast id>{v}</Toast> }
            />
        </div>
    }
}

pub fn toast(v: impl IntoView) {
    let toaster: RwSignal<Toaster> = expect_context();

    toaster.update(move |toaster| {
        toaster.toasts.push((toaster.id, v.into_view()));
        toaster.id = toaster.id.wrapping_add(1);
    });
}

#[derive(Debug, Default, Clone)]
struct Toaster {
    toasts: Vec<(usize, View)>,
    id: usize,
}

impl Toaster {
    fn remove(&mut self, id: usize) {
        let Some(i) = self
            .toasts
            .iter()
            .enumerate()
            .find_map(|(i, (toast_id, _))| (id == *toast_id).then_some(i))
        else {
            return;
        };
        let _ = self.toasts.remove(i);
    }
}

#[component]
fn Toast(id: usize, children: Children) -> impl IntoView {
    let node_ref = create_node_ref::<html::Div>();
    let list: RwSignal<Toaster> = expect_context();
    let close = move || list.update(move |list| list.remove(id));
    let hover = use_element_hover(node_ref);

    let UseTimeoutFnReturn {
        start,
        stop,
        is_pending,
        ..
    } = {
        let close = close.clone();
        use_timeout_fn(move |_| close(), 4.0 * 1000.0)
    };

    create_effect(move |_| {
        if !hover() {
            start(());
            return;
        }

        if is_pending() {
            stop();
        }
    });

    view! {
        <style>{STYLE_SHEET}</style>
        <div
            node_ref=node_ref
            class=move || {
                tw_join!(
                    ClassName::SHOW_UP,
                    "z-10 flex flex-row justify-between p-2 text-text bg-slate-800 border-2 border-secondary mr-2 mb-2",
                    "before:bg-primary before:contents before:absolute before:bottom-0 before:right-0 before:h-1 before:w-full",
                    is_pending().then_some(ClassName::COUNTDOWN)
                )
            }
        >

            <div class="text-sm">{children()}</div>
            <button class="w-6 h-6 pl-2" on:click=move |_| close()>
                <Icon icon=icondata::AiCloseOutlined/>
            </button>
        </div>
    }
}
