use std::time::Duration;

use leptos::*;
use leptos_animated_for::AnimatedFor;
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
        <div class="fixed bottom-0 right-0 h-screen min-w-64 w-1/5 flex flex-col justify-end">
            <AnimatedFor
                each=toasts
                key=|toast| toast.0
                children=|(id, v)| view! { <Toast id>{v}</Toast> }
                enter_from_class="translate-x-full"
                enter_class="translate-x-0 transition-transform duration-300"
                move_class="transition-all duration-300"
                leave_class="[&>div:last-child]:hidden translate-x-full transition-transform duration-300"
                appear=true
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
    let list: RwSignal<Toaster> = expect_context();
    let close = move || list.update(move |list: &mut Toaster| list.remove(id));
    let UseTimeoutFnReturn {
        start,
        stop,
        is_pending,
        ..
    } = {
        let close = close.clone();
        use_timeout_fn(move |_| close(), 4.0 * 1000.0)
    };

    let node_ref = create_node_ref::<html::Div>();
    let hover = use_element_hover(node_ref);
    create_effect(move |_| {
        if !hover() {
            start(());
            return;
        }

        if is_pending() {
            stop();
        }
    });
    let countdown_class = move || {
        tw_join!(
            "w-full h-0 relative before:absolute before:bottom-0 \
             before:right-0 before:h-1 before:w-full before:bg-primary",
            is_pending().then_some(ClassName::COUNTDOWN),
        )
    };
    view! {
        <div node_ref=node_ref class="pr-2 pb-2">
            <div class="z-10 flex flex-row justify-between p-2 text-text bg-slate-800 border-2 border-secondary ">
                <div class="text-sm">{children()}</div>
                <button class="w-6 h-6 pl-2" on:click=move |_| close()>
                    <Icon icon=icondata::AiCloseOutlined/>
                </button>
            </div>
            <style>{STYLE_SHEET}</style>
            <div class=countdown_class></div>
        </div>
    }
}
