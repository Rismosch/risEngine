#pragma once
#include <iostream>
#include "../risData/risData.h"

namespace risUtility
{
	using namespace risData;

	class FlagModule
	{
	public:
		FlagModule();
		~FlagModule();
		
		void apply(U64 flags) const;
		U64 retrieve() const;
		
		bool get(U8 flag) const;
		void set(U8 flag, bool value) const;
		void toggle(U8 flag) const;

		std::string toString() const;
	private:
		struct Impl;
		Impl* pImpl;
	};
}