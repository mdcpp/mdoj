use leptos::*;
use leptos_animated_for::AnimatedFor;
use leptos_icons::*;
use leptos_use::*;
use tailwind_fuse::*;

turf::style_sheet!("src/components/toast.scss");

#[component]
pub fn ProvideToast(children: Children) -> impl IntoView {
    let toaster = create_rw_signal(Toaster::default());
    provide_context(toaster);
    let (toasts, _) = slice!(toaster.toasts);
    view! {
        {children()}
        <div class="fixed bottom-0 right-0 h-fit min-w-64 w-1/5 flex flex-col justify-end">
            <AnimatedFor
                each=toasts
                key=|toast| toast.0
                children=|(id, variant, v)| {
                    view! {
                        <Toast id variant>
                            {v}
                        </Toast>
                    }
                }

                enter_from_class="translate-x-full"
                enter_class="translate-x-0 transition-transform duration-300"
                move_class="transition-all duration-300"
                leave_class="[&>div:last-child]:hidden translate-x-full transition-transform duration-300"
                appear=true
            />
        </div>
    }
}

pub fn use_toast() -> impl Fn(ToastVariant, View) {
    let toaster: RwSignal<Toaster> = expect_context();

    move |variant, v| {
        toaster.update(move |toaster| {
            toaster.toasts.push((toaster.id, variant, v));
            toaster.id = toaster.id.wrapping_add(1);
        });
    }
}

#[derive(Debug, Default, Clone)]
struct Toaster {
    toasts: Vec<(usize, ToastVariant, View)>,
    id: usize,
}

impl Toaster {
    fn remove(&mut self, id: usize) {
        let Some(i) = self
            .toasts
            .iter()
            .enumerate()
            .find_map(|(i, (toast_id, ..))| (id == *toast_id).then_some(i))
        else {
            logging::error!("cannot remove id `{id}`");
            return;
        };
        let _ = self.toasts.remove(i);
    }
}

#[derive(Debug, TwVariant, PartialEq, Eq)]
pub enum ToastVariant {
    #[tw(default, class = "border-secondary")]
    Info,
    #[tw(class = "border-green-500")]
    Success,
    #[tw(class = "border-yellow-300")]
    Warning,
    #[tw(class = "border-red-500")]
    Error,
}

#[derive(Debug, TwVariant, PartialEq, Eq)]
enum CountdownVariant {
    #[tw(default, class = "before:bg-secondary")]
    Info,
    #[tw(class = "before:bg-green-500")]
    Success,
    #[tw(class = "before:bg-yellow-300")]
    Warning,
    #[tw(class = "before:bg-red-500")]
    Error,
}

#[component]
fn Toast(
    id: usize,
    variant: ToastVariant,
    children: Children,
) -> impl IntoView {
    let list: RwSignal<Toaster> = expect_context();
    let close = move || list.update(move |list: &mut Toaster| list.remove(id));
    let UseTimeoutFnReturn {
        start,
        stop,
        is_pending,
        ..
    } = {
        let close = close;
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

    let countdown_variant = match variant {
        ToastVariant::Info => CountdownVariant::Info,
        ToastVariant::Success => CountdownVariant::Success,
        ToastVariant::Warning => CountdownVariant::Warning,
        ToastVariant::Error => CountdownVariant::Error,
    };

    let countdown_class = move || {
        tw_join!(
            "w-full h-0 relative before:absolute before:bottom-0 \
             before:right-0 before:h-1 before:w-full",
            countdown_variant,
            is_pending().then_some(ClassName::COUNTDOWN),
        )
    };

    view! {
        <div node_ref=node_ref class="pr-2 pb-2">
            <div class=tw_join!(
                "z-10 flex flex-row justify-between p-2 text-text bg-black-800 border-2 border-b-0",
                variant
            )>
                <div class="text-sm">{children()}</div>
                <button class="size-6 pl-2" on:click=move |_| close()>
                    <Icon icon=icondata::AiCloseOutlined />
                </button>
            </div>
            <style>{STYLE_SHEET}</style>
            <div class=countdown_class></div>
        </div>
    }
}
