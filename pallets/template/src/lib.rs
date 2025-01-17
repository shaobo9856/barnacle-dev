#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;


#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::traits::UnixTime;
	use frame_support::pallet_prelude::{*, ValueQuery};
	use frame_support::inherent::Vec;
	use frame_system::pallet_prelude::*;
	use frame_support::sp_runtime::SaturatedConversion;
	use frame_support::{
		traits::{
			Currency, ExistenceRequirement::KeepAlive, Get, Imbalance, OnUnbalanced,
			ReservableCurrency, WithdrawReasons,
		},
		weights::Weight,
		PalletId,
	};
	pub type BalanceOf<T> = <<T as Config>::MyCurrency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	/// To represent a rendering instance
	#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, TypeInfo, Debug)]
	pub struct Renderer<BlockNumber> {
		// ipv6 or ipv4(u32)
		pub host: Vec<u8>,
		pub portoffset: u16,
		pub reg_at: BlockNumber,
		pub status: u8,
		pub gameid: Vec<u8>,
		// TODO POV staking
	}

	// /// To represent a order
	// #[derive(Encode, Decode, Clone, PartialEq, Eq, Default, TypeInfo, Debug)]
	// pub struct Order{
	// 	pub player: Vec<u8>,
	// 	pub starttime: u16,
	// }

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type MyCurrency: Currency<Self::AccountId>;
		type TimeProvider: UnixTime;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	// // The pallet's runtime storage items.
	// // https://docs.substrate.io/v3/runtime/storage
	// #[pallet::storage]
	// #[pallet::getter(fn something2)]
	// pub type Something2<T:Config> = StorageDoubleMap<_,Blake2_128Concat, u32, Blake2_128Concat, T::AccountId, u32,ValueQuery>;

	/// Storage of all renderers.
    /// The storage does not guarante that the host:port of a renderer is unique,
    /// some extra constraints must be applied
    #[pallet::storage]
	#[pallet::getter(fn renderers)]
    pub type Renderers<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        u32,
        Blake2_128Concat,
        T::AccountId,
        Renderer<T::BlockNumber>,
        OptionQuery,
    >;

	#[pallet::storage]
	#[pallet::getter(fn orders)]
	// Learn more about declaring storage items:
	// https://docs.substrate.io/v3/runtime/storage#declaring-storage-items
	pub type Orders<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		T::AccountId,
		u64, //Starttime,
		OptionQuery,
	>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		SomethingStored(u32, T::AccountId),
		RendererRegistered(T::AccountId),
		RendererDeregistered(T::AccountId),
		RendererConnected(T::AccountId,T::AccountId,u64),
		RendererDisconnected(T::AccountId,T::AccountId,u64),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
		AlreadyRegistered,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		// #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		// pub fn do_something2(origin: OriginFor<T>, something1: u32, something2: u32) -> DispatchResult {
		// 	let who = ensure_signed(origin)?;
		// 	// Update storage.
		// 	Something2::<T>::insert(something1,who.clone(),something2);
		// }
		// #[pallet::weight(10_000)]
		// pub fn my_function(origin: OriginFor<T>, dest: T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
		// 	let buyer = ensure_signed(origin)?;
		// 	T::MyCurrency::transfer(&buyer, &dest, amount, KeepAlive)?;
		// 	Ok(())
		// }

		/// Register a renderer. A basic challenge should be initiated immediately by the offchain workers.
		#[pallet::weight(10_000)]
		pub fn register(origin: OriginFor<T>, region: u32, host: Vec<u8>, portoffset: u16, games: Vec<u8>) -> DispatchResultWithPostInfo {
			let renderer = ensure_signed(origin)?;
			Renderers::<T>::try_mutate(&region, &renderer, |exists| -> DispatchResult {
				ensure!(exists.is_none(), Error::<T>::AlreadyRegistered);
				exists.replace(Renderer {
					host,
					portoffset,
					reg_at: frame_system::Pallet::<T>::block_number(),
					status: 0,
					gameid: games,
				});
				Ok(())
			})?;
			Self::deposit_event(Event::RendererRegistered(renderer));
			Ok(().into())
		}


		/// deregister a renderer. 
		#[pallet::weight(10_000)]
		pub fn deregister(origin: OriginFor<T>, region: u32) -> DispatchResultWithPostInfo {
			let renderer = ensure_signed(origin)?;
			Renderers::<T>::remove(&region, &renderer);
			Self::deposit_event(Event::RendererDeregistered(renderer));
			Ok(().into())
		}


		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn connect(
			origin: OriginFor<T>,region: u32,
			renderer: T::AccountId,
			// TODO add stablecoin payment
		) -> DispatchResult {
			let player = ensure_signed(origin)?;
			Renderers::<T>::try_mutate(&region, &renderer, |exists| -> DispatchResult {
				ensure!(!exists.is_none(), Error::<T>::AlreadyRegistered);
				exists.replace(Renderer {
					host:exists.as_ref().unwrap().host.clone(),
					portoffset:exists.as_ref().unwrap().portoffset,
					reg_at: exists.as_ref().unwrap().reg_at,
					status: 1,
					gameid: exists.as_ref().unwrap().gameid.clone(),
				});
				Ok(())
			})?;
			// TODO pay for the connection
			let starttime = T::TimeProvider::now().as_secs();
			Orders::<T>::try_mutate(&renderer, &player, |exists| -> DispatchResult {
				ensure!(exists.is_none(), Error::<T>::AlreadyRegistered);
				exists.replace(starttime);
				Ok(())
			})?;

			Self::deposit_event(Event::RendererConnected(renderer,player,starttime));
			Ok(())
		}


		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn disconnect(origin: OriginFor<T>, region: u32, renderer: T::AccountId) -> DispatchResult {
			let player = ensure_signed(origin)?;
			Renderers::<T>::try_mutate(&region, &renderer, |exists| -> DispatchResult {
				ensure!(!exists.is_none(), Error::<T>::AlreadyRegistered);
				exists.replace(Renderer {
					host:exists.as_ref().unwrap().host.clone(),
					portoffset:exists.as_ref().unwrap().portoffset,
					reg_at: exists.as_ref().unwrap().reg_at,
					status: 0,
					gameid: exists.as_ref().unwrap().gameid.clone(),
				});
				Ok(())
			})?;
			// TODO pay for the connection	
			let starttime: u64 = Orders::<T>::get(&renderer, &player).unwrap();
			let endtime: u64 = T::TimeProvider::now().as_secs();
			let duration: u64 = endtime - starttime;
			let price: u64 = 10000000000000;
			let amount: u64 = duration * price;
			let dest = renderer.clone();
			T::MyCurrency::transfer(&player, &dest, amount.saturated_into::<BalanceOf<T>>(), KeepAlive)?;
			Orders::<T>::remove(&renderer, &player);
			Self::deposit_event(Event::RendererDisconnected(renderer,player,endtime));
			Ok(())
		}

	}

}
